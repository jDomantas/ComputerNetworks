// #include <sys/socket.h>
// #include <sys/select.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "network.h"
#include "reporting.h"
#include "server.h"

static bool parseLong(const char *input, long *result) {
	char *ep;
	*result = strtol(input, &ep, 0);
	return *ep == 0;
}

static void printUsage(const char *name) {
	printf("usage:\n");
	printf("  %s <listen port>\n", name);
	exit(1);
}

int main(int argc, const char **argv) {
	if (argc != 2) {
		printUsage(argv[0]);
	}
	
	long port;
	if (!parseLong(argv[1], &port) || port < 1 || port > 65535) {
		reportError("Invalid port, must be a number in range 1 - 65535", false);
	}

	ServerCallbacks callbacks = { &onMessage, &onConnected, &onDisconnected };

	Server server = createServer((uint16_t)port, callbacks);

	printMessage("Started server, listening on port %d", port);
	
	while (true) {
		serverTick(&server);
	}
}

