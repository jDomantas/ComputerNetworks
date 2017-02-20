#include <errno.h>
#include <string.h>
#include <time.h>
#include <stdlib.h>
#include <stdio.h>
#include "reporting.h"

void reportError(const char *msg, bool checkErrno) {
	int err = errno;
	fprintf(stderr, "%s\n", msg);
	if (checkErrno) {
		fprintf(stderr, "%s (error code: %d)\n", strerror(err), err);
	}
	exit(1);
}
