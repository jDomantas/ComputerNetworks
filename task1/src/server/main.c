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

static void onMessage(Server *server, Client *client, const char *msg) {
	char buffer[MAX_MESSAGE_LENGTH + MAX_NAME_LENGTH + 20];
	sprintf(buffer, "%s%s>%s %s",
		colorEscapeSequence(Yellow),
		client->name,
		colorEscapeSequence(None),
		msg);
	sendToAll(server, buffer);
	printMessage("%s", buffer);
}

static void onConnected(Server *server, Client *client) {
	printColoredMessage(Yellow, "%s connected", client->name);
	char msg[MAX_NAME_LENGTH + 50];
	sprintf(msg, "%s%s connected%s",
		colorEscapeSequence(Yellow),
		client->name,
		colorEscapeSequence(None));
	sendToAll(server, msg);
}

static void onDisconnected(Server *server, Client *client) {
	printColoredMessage(Yellow, "%s disconnected", client->name);
	char msg[MAX_NAME_LENGTH + 50];
	sprintf(msg, "%s%s disconnected%s",
		colorEscapeSequence(Yellow),
		client->name,
		colorEscapeSequence(None));
	sendToAll(server, msg);
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

