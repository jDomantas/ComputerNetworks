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

static const char *colorEscapeSequence(Color c) {
	switch (c) {
		case Red:     return "\x1B[31;1m";
		case Green:   return "\x1B[32;1m";
		case Yellow:  return "\x1B[33;1m";
		case Blue:    return "\x1B[34;1m";
		case Magenta: return "\x1B[35;1m";
		case Cyan:    return "\x1B[36;1m";
		case White:   return "\x1B[37;1m";
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
	puts("\x1B[0m");
	va_end(args);
}
