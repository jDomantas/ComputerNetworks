#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include "reporting.h"
#include "network.h"
#include "server.h"

static void displayMessage(Server *server, const char *format, ...) {
	char buffer[MAX_MESSAGE_LENGTH];
	
	va_list v;
	va_start(v, format);
	vsnprintf(buffer, MAX_MESSAGE_LENGTH, format, v);
	buffer[MAX_MESSAGE_LENGTH - 1] = 0;
	va_end(v);
	
	printMessage(buffer);
	sendToAll(server, buffer);
}

void onMessage(Server *server, Client *client, const char *msg) {
	displayMessage(server, "%s%s>%s %s",
		yellow,
		client->name,
		none,
		msg);
}

void onConnected(Server *server, Client *client) {
	displayMessage(server, "%s%s connected%s", yellow, client->name, none);
}

void onDisconnected(Server *server, Client *client) {
	displayMessage(server, "%s%s disconnected%s", yellow, client->name, none);
}
