#include <sys/select.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include "reporting.h"
#include "network.h"

Server createServer(uint16_t port, ClientMessageCallback callback) {
	Server server;
	
	server.port = port;
	server.callback = callback;
	for (int i = 0; i < MAX_CLIENTS; i++) {
		server.clients[i].isConnected = false;
	}

	// create server socket
	if ((server.socket = socket(AF_INET, SOCK_STREAM, 0)) < 0) {
		reportError("Unable to create socket", true);
	}

	// set server address & shit
	memset(&server.address, 0, sizeof(server.address));
	server.address.sin_family = AF_INET;
	server.address.sin_addr.s_addr = htonl(INADDR_ANY); 
	server.address.sin_port = htons(port);

	// bind socket to address
	if (bind(server.socket, (struct sockaddr*)&server.address, sizeof(server.address)) < 0) {
		reportError("Unable to bind listening socket", true);
	}

	if (listen(server.socket, 5) < 0) {
		reportError("Error in listen()", true);
	}

	return server;
}

static Client *getClientSlot(Server *server) {
	for (int i = 0; i < MAX_CLIENTS; i++) {
		if (!server->clients[i].isConnected) {
			return &(server->clients[i]);
		}
	}
	
	return NULL;
}

static void acceptClient(Server *server) {
	Client *c = getClientSlot(server);
	if (c == NULL) {
		// maybe do something about it?
		return;
	}

	memset(&(c->address), 0, sizeof(c->address));
	socklen_t addressSize = (socklen_t)sizeof(c->address);
	c->socket = accept(server->socket, (struct sockaddr*)&(c->address), &addressSize);
	printColoredMessage(Yellow, "Client connected: %s", inet_ntoa(c->address.sin_addr));

	c->isConnected = true;
	c->lastPingTime = time(NULL);
	c->nextMessageLength = 0;
	strcpy(c->name, "User");
}

static void clientDisconnected(Server *server, Client *client) {
	printColoredMessage(Yellow, "%s disconnected", client->name);
	client->isConnected = false;
}

static void readClientMessage(Server *server, Client *client) {
	if (client->nextMessageLength == 0) {
		// client should prepend the message length to the message,
		uint32_t messageSize;
		ssize_t bytesRead = recv(client->socket, &messageSize, 4, MSG_PEEK);
		if (bytesRead == 0) {
			// client disconnected
			clientDisconnected(server, client);
		} else if (bytesRead == -1) {
			// error happened while reading from socket, maybe report it?
		} else if (bytesRead == 4) {
			// consume those bytes that were only peeked at
			recv(client->socket, &messageSize, 4, 0);
			// got proper message size, store it
			client->nextMessageLength = ntohl(messageSize);
			// client proved that he is alive, store current time
			client->lastPingTime = time(NULL);
		}
	} else {
		if (client->nextMessageLength > MAX_MESSAGE_LENGTH) {
			// client is evil because he tries to send too long messages, what to do?
			printColoredMessage(Red, "Client is evil, message length: %zd", client->nextMessageLength);
		} else {
			char message[MAX_MESSAGE_LENGTH + 1];
			ssize_t bytesRead = recv(client->socket, message, client->nextMessageLength, MSG_PEEK);
			if (bytesRead == -1) {
				// error happened while reading from socket, maybe report it?
			} else if (bytesRead == 0) {
				// client disconnected
				clientDisconnected(server, client);
			} else if ((size_t)bytesRead == client->nextMessageLength) {
				// got all message, consume it
				recv(client->socket, message, client->nextMessageLength, 0);
				message[client->nextMessageLength] = 0;
				client->nextMessageLength = 0;
				// and execute callback with client message
				if (server->callback != NULL) {
					server->callback(server, client, message);
				} else {
					printMessage("%s> %s", client->name, message);
				}
			}
		}
	}
}

static void sendRawMessage(Server *server, Client *client, const char *msg, uint32_t msgSize) {
	if (!client->isConnected) {
		return;
	}
	
	uint32_t size = htonl(msgSize);
	ssize_t bytesWritten = send(client->socket, &size, 4, 0);
	if (bytesWritten <= 0) {
		clientDisconnected(server, client);
		return;
	}
	
	if (msgSize > 0) {
		bytesWritten = send(client->socket, msg, msgSize, 0);
		if (bytesWritten <= 0) {
			clientDisconnected(server, client);
		}
	}
}

void sendMessage(Server *server, Client *client, const char *msg) {
	size_t msgSize = strlen(msg);
	if (msgSize > MAX_MESSAGE_LENGTH) {
		// message is very long, just truncate it
		msgSize = MAX_MESSAGE_LENGTH;
	}
	
	// MAX_MESSAGE_LENGTH should fit in uint32_t, please?
	sendRawMessage(server, client, msg, (uint32_t)msgSize);
}

void sendToAll(Server *server, const char *msg) {
	for (int i = 0; i < MAX_CLIENTS; i++) {
		sendMessage(server, &server->clients[i], msg);
	}
}

void serverTick(Server *server) {
	fd_set set;
	FD_ZERO(&set);

	int maxFd = -1;

	for (int i = 0; i < MAX_CLIENTS; i++) {
		if (server->clients[i].isConnected) {
			FD_SET(server->clients[i].socket, &set);
			if (server->clients[i].socket > maxFd) {
				maxFd = server->clients[i].socket;
			}
		}
	}

	FD_SET(server->socket, &set);
	if (server->socket > maxFd) {
		maxFd = server->socket;
	}

	select(maxFd + 1, &set, NULL, NULL, NULL);
	
	if (FD_ISSET(server->socket, &set)) {
		acceptClient(server);
	}
	
	for (int i = 0; i < MAX_CLIENTS; i++) {
		if (server->clients[i].isConnected &&
			FD_ISSET(server->clients[i].socket, &set)) {
			readClientMessage(server, &server->clients[i]);
		}
	}
}
