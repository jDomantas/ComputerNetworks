#ifndef REPORTING_H
#define REPORTING_H

#include <stdbool.h>

typedef enum {
	Red,
	Green,
	Yellow,
	Blue,
	Magenta,
	Cyan,
	White,
} Color;

void reportError(const char *msg, bool checkErrno);
void printMessage(const char *format, ...);
void printColoredMessage(Color color, const char *format, ...);

#endif
