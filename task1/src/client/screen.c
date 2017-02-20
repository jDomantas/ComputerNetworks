#include <ncurses.h>
#include <string.h>
#include "screen.h"
#include "network.h"

#define MAX_MESSAGES 80

static char messages[MAX_MESSAGES][MAX_MESSAGE_LENGTH];
static int messageCount = 0;
static char input[MAX_MESSAGE_LENGTH];
static int currentInputPos = 0;
static int screenWidth;
static int screenHeight;
static bool shouldResetInput;

static int escapeSequenceType(const char *line) {
	if (line[0] != '\x1B' || line[1] != '[') {
		return -1;
	}
	
	// 0m - reset sequence
	line += 2;
	if (line[0] == '0' && line[1] == 'm')
		return 0;
	
	// 3xm - set color sequence
	if (line[0] == '3' && line[1] >= '1' && line[1] <= '7' && line[2] == 'm') {
		return line[1] - '0';
	}
	
	// 3x;1m - set bright color sequence
	if (line[0] == '3' &&
		line[1] >= '1' && line[1] <= '7' &&
		line[2] == ';' &&
		line[3] == '1' &&
		line[4] == 'm') {
		return line[1] - '0' + 8;
	}
	
	return -1;
}

static size_t lineLength(const char *line) {
	size_t pos = 0;
	size_t skipped = 0;
	while (line[pos] != 0) {
		int sequence = escapeSequenceType(line + pos);
		if (sequence == 0) {
			pos += 4;
			skipped += 4;
		} else if (sequence >= 1 && sequence < 8) {
			pos += 5;
			skipped += 5;
		} else if (sequence >= 8 && sequence < 16) {
			pos += 7;
			skipped += 7;
		} else {
			pos++;
		}
	}
	
	return pos - skipped;
}

static void printText(int row, int col, const char *text) {
	size_t curr = 0;
	int x = col;
	int currColor = 0;
	while (text[curr] != 0) {
		int sequence = escapeSequenceType(text + curr);
		if (sequence == -1) {
			mvaddch(row, x, text[curr]);
			x++;
			curr++;
			if (x > screenWidth) {
				x = col;
				row++;
			}
		} else if (sequence == 0) {
			curr += 4;
			if (currColor != 0) {
				attroff(COLOR_PAIR(currColor));
				currColor = 0;
			}
		} else {
			curr += 5 + 2 * ((size_t)sequence / 8);
			sequence %= 8;
			if (currColor != 0) {
				attroff(COLOR_PAIR(currColor));
				currColor = 0;
			}
			attron(COLOR_PAIR(currColor = sequence));
		}
	}
}

static void drawInput() {
	int inputWidth = screenWidth - 2;
	int inputLines = (currentInputPos + inputWidth - 1) / inputWidth;
	if (inputLines < 1) {
		inputLines = 1;
	}
	printText(screenHeight - inputLines, 0, "> ");
	printText(screenHeight - inputLines, 2, input);
	move(screenHeight - 1, 2 + currentInputPos % inputWidth);
}

static void drawMessages() {
	int currentLine = screenHeight - 3;
	for (int i = 0; i < messageCount; i++) {
		int messageLength = (int)lineLength(messages[i]);
		int lines = (messageLength + screenWidth - 1) / screenWidth;
		printText(currentLine - lines + 1, 0, messages[i]);
		currentLine -= lines;
	}
}

static void redrawScreen() {
	getmaxyx(stdscr, screenHeight, screenWidth);
	clear();
	drawMessages();
	drawInput();
	refresh();
}

void addLine(const char *line) {
	for (int i = MAX_MESSAGES - 2; i >= 0; i--)
		memcpy(messages[i + 1], messages[i], MAX_MESSAGE_LENGTH);
	strncpy(messages[0], line, MAX_MESSAGE_LENGTH - 1);
	messages[0][MAX_MESSAGE_LENGTH - 1] = 0;
	
	messageCount++;
	if (messageCount > MAX_MESSAGES) {
		messageCount = MAX_MESSAGES;
	}
	
	redrawScreen();
}

void initScreen() {
	initscr();
	start_color();
	timeout(1);
	cbreak();
	noecho();
	
	for (short i = 1; i < 8; i++)
		init_pair(i, i, 0);

	messageCount = 0;
	currentInputPos = 0;
	shouldResetInput = false;
	
	redrawScreen();
}

const char *getInput() {
	if (shouldResetInput) {
		shouldResetInput = false;
		currentInputPos = 0;
		input[0] = 0;
		redrawScreen();
	}
	
	int c = getch();
	if (c != ERR) {
		if (c >= 32 && c < 127 && currentInputPos < MAX_MESSAGE_LENGTH - 1) {
			input[currentInputPos++] = (char)c;
			input[currentInputPos] = 0;
			redrawScreen();
		} else if (c == 127 && currentInputPos > 0) {
			input[--currentInputPos] = 0;
			redrawScreen();
		} else if (c == 10) {
			shouldResetInput = true;
			return input; 
		}
	}
	
	return NULL;
}

void closeScreen() {
	endwin();
}
