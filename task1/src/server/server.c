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

static void displayPrivateMessage(Server *server, Client *client, const char *format, ...) {
	char buffer[MAX_MESSAGE_LENGTH + MAX_NAME_LENGTH + 10];

	strcpy(buffer, "To: ");
	strcpy(buffer + 4, client->name);
	size_t nameLength = strlen(client->name);
	strcpy(buffer + 4 + nameLength, " - ");

	va_list v;
	va_start(v, format);
	vsnprintf(buffer + nameLength + 7, MAX_MESSAGE_LENGTH, format, v);
	buffer[sizeof(buffer) - 1] = 0;
	va_end(v);

	sendMessage(server, client, buffer + nameLength + 7);
	printColoredMessage(Magenta, buffer);
}

static bool isCommand(const char *command, const char *msg, const char **argStart, size_t *len) {
	size_t commandLength = strlen(command);

	if (strncmp(msg, command, commandLength) != 0) {
		return false;
	}

	if (msg[commandLength] != ' ') {
		return false;
	}

	size_t args = commandLength;
	while (msg[args] == ' ') {
		args++;
	}

	size_t lastNonSpace = args;
	for (size_t pos = lastNonSpace; msg[pos] != 0; pos++) {
		if (msg[pos] != ' ') {
			lastNonSpace = pos;
		}
	}

	*argStart = msg + args;
	*len = lastNonSpace - args + 1;
	return true;
}

static void executeCommand(Server *server, Client *client, const char *command) {
	const char *argStart;
	size_t argLen;
	if (isCommand("name", command, &argStart, &argLen)) {
		if (argLen > MAX_NAME_LENGTH) {
			displayPrivateMessage(server, client,
				"Name cannot be longer than %d charactars",
				MAX_NAME_LENGTH);
		} else {
			char oldName[MAX_NAME_LENGTH];
			strcpy(oldName, client->name);
			memcpy(client->name, argStart, argLen);
			client->name[argLen] = 0;
			displayMessage(server, "%s%s is now %s%s",
				yellow,
				oldName,
				client->name,
				none);
		}
	} else if (isCommand("me", command, &argStart, &argLen)) {
		displayMessage(server, "%s%s %.*s%s",
			blue,
			client->name,
			argLen,
			argStart,
			none);
	}
	#define colorCommand(colorName, color) \
		else if (isCommand((colorName), command, &argStart, &argLen)) { \
			displayMessage(server, "%s%s%s> %s%.*s%s", \
				yellow, \
				client->name, \
				none, \
				(color), \
				argLen, \
				argStart, \
				none); \
		}
	colorCommand("red", red)
	colorCommand("green", green)
	colorCommand("blue", blue)
	colorCommand("cyan", cyan)
	colorCommand("magenta", magenta)
	colorCommand("yellow", yellow)
	#undef colorCommand
	else {
		displayPrivateMessage(server, client, "Unknown command");
	}
}

void onMessage(Server *server, Client *client, const char *msg) {
	if (msg[0] == '/') {
		executeCommand(server, client, msg + 1);
	} else {
		displayMessage(server, "%s%s>%s %s",
			yellow,
			client->name,
			none,
			msg);
	}
}

void onConnected(Server *server, Client *client) {
	displayMessage(server, "%s%s connected%s", yellow, client->name, none);
}

void onDisconnected(Server *server, Client *client) {
	displayMessage(server, "%s%s disconnected%s", yellow, client->name, none);
}
