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
	None,
} Color;

static const char *red     = "\x1B[31;1m";
static const char *green   = "\x1B[32;1m";
static const char *blue    = "\x1B[34;1m";
static const char *yellow  = "\x1B[33;1m";
static const char *magenta = "\x1B[35;1m";
static const char *cyan    = "\x1B[36;1m";
static const char *white   = "\x1B[37;1m";
static const char *none    = "\x1B[0m";

const char *colorEscapeSequence(Color color);
void reportError(const char *msg, bool checkErrno);
void printMessage(const char *format, ...);
void printColoredMessage(Color color, const char *format, ...);

#endif
