// #include <sys/socket.h>
// #include <sys/select.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "network.h"
#include "reporting.h"

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

static void onClientMessage(Server *server, Client *client, const char *msg) {
	char buffer[MAX_MESSAGE_LENGTH + MAX_NAME_LENGTH + 2];
	strcpy(buffer, client->name);
	size_t writePos = strlen(client->name);
	buffer[writePos++] = '>';
	buffer[writePos++] = ' ';
	strcpy(buffer + writePos, msg);
	sendToAll(server, buffer);
	printMessage("%s", buffer);
}

int main(int argc, const char **argv) {
	if (argc != 2) {
		printUsage(argv[0]);
	}
	
	long port;
	if (!parseLong(argv[1], &port) || port < 1 || port > 65535) {
		reportError("Invalid port, must be a number in range 1 - 65535", false);
	}

	Server server = createServer((uint16_t)port, &onClientMessage);

	printMessage("Started server, listening on port %d", port);
	
	while (true) {
		serverTick(&server);
	}
}

