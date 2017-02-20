#include <errno.h>
#include <string.h>
#include <time.h>
#include <stdlib.h>
#include <stdio.h>
#include "reporting.h"

static void printCurrentTime() {
	char formattedTime[100];
	time_t currentTime = time(NULL);
	struct tm *info = localtime(&currentTime);
	strftime(formattedTime, sizeof(formattedTime), "%H:%M:%S", info);
	formattedTime[sizeof(formattedTime) - 1] = 0;
	printf("[%s] ", formattedTime);
}

const char *colorEscapeSequence(Color c) {
	switch (c) {
		case Red:     return red;
		case Green:   return green;
		case Yellow:  return yellow;
		case Blue:    return blue;
		case Magenta: return magenta;
		case Cyan:    return cyan;
		case White:   return white;
		case None:    return none;
	}
}

void reportError(const char *msg, bool checkErrno) {
	int err = errno;
	fprintf(stderr, "%s\n", msg);
	if (checkErrno) {
		fprintf(stderr, "%s (error code: %d)\n", strerror(err), err);
	}
	exit(1);
}

void printMessage(const char *format, ...) {
	printCurrentTime();
	
	va_list args;
	va_start(args, format);
	vprintf(format, args);
	puts("");
	va_end(args);
}

void printColoredMessage(Color color, const char *format, ...) {
	printCurrentTime();
	
	printf("%s", colorEscapeSequence(color));
	va_list args;
	va_start(args, format);
	vprintf(format, args);
	puts(colorEscapeSequence(None));
	va_end(args);
}
