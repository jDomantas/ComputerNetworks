#include <sys/select.h>
#include <sys/socket.h>
#include <arpa/inet.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "network.h"
#include "reporting.h"


Client createClient(const char *address, uint16_t port, MessageCallback callback) {
	Client client;
	
	client.state = Connected;
	client.callback = callback;
	
	// create socket
	if ((client.socket = socket(AF_INET, SOCK_STREAM, 0)) < 0){
		reportError("Cannot create socket", true);
	}
								
   	// set server properties
	memset(&client.serverAddress, 0, sizeof(client.serverAddress));
	client.serverAddress.sin_family = AF_INET; // protocol
	client.serverAddress.sin_port = htons(port); // port
	
	// parse and set server address
	// clang complains that it cannot find inet_aton
	// but the binary still compiles...  :( ??
	if (inet_aton(address, &client.serverAddress.sin_addr) <= 0) {
		reportError("Invalid remote IP address", true);
	}

	// connect to server
	if (connect(
			client.socket,
			(struct sockaddr*)&client.serverAddress,
			sizeof(client.serverAddress)) < 0) {
		reportError("Could not connect to server", true);
	}
	
	client.lastMessageTime = time(NULL);
	client.nextServerMessageLength = 0;
	return client;
} 

static void connectionError(Client *client) {
	client->state = Error;
}

static void disconnected(Client *client) {
	client->state = LostConnection;
}

static void readMessage(Client *client) {
	if (client->nextServerMessageLength == 0) {
		uint32_t messageSize;
		ssize_t bytesRead = recv(client->socket, &messageSize, 4, MSG_PEEK);
		if (bytesRead == 0) {
			// client disconnected
			disconnected(client);
		} else if (bytesRead == -1) {
			// error happened while reading from socket, maybe report it?
		} else if (bytesRead == 4) {
			// consume those bytes that were only peeked at
			recv(client->socket, &messageSize, 4, 0);
			// got proper message size, store it
			client->nextServerMessageLength = ntohl(messageSize);
			// client proved that he is alive, store current time
			client->lastMessageTime = time(NULL);
		}
	} else {
		if (client->nextServerMessageLength > MAX_MESSAGE_LENGTH) {
			// server is evil because he tries to send too long messages, what to do?
			connectionError(client);
		} else {
			char message[MAX_MESSAGE_LENGTH + 1];
			ssize_t bytesRead = recv(client->socket, message, client->nextServerMessageLength, MSG_PEEK);
			if (bytesRead == -1) {
				// error happened while reading from socket, maybe report it?
			} else if (bytesRead == 0) {
				// client disconnected
				disconnected(client);
			} else if ((size_t)bytesRead == client->nextServerMessageLength) {
				// got all message, consume it
				recv(client->socket, message, client->nextServerMessageLength, 0);
				message[client->nextServerMessageLength] = 0;
				client->nextServerMessageLength = 0;
				// and execute callback with client message
				if (client->callback != NULL) {
					client->callback(client, message);
				}
			}
		}
	}
}

static void sendRawMessage(Client *client, const char *msg, uint32_t msgSize) {
	if (client->state != Connected) {
		return;
	}
	
	uint32_t size = htonl(msgSize);
	ssize_t bytesWritten = send(client->socket, &size, 4, 0);
	if (bytesWritten <= 0) {
		disconnected(client);
		return;
	}
	
	if (msgSize > 0) {
		bytesWritten = send(client->socket, msg, msgSize, 0);
		if (bytesWritten <= 0) {
			disconnected(client);
		}
	}
}

void sendMessage(Client *client, const char *msg) {
	size_t msgSize = strlen(msg);
	if (msgSize > MAX_MESSAGE_LENGTH) {
		// message is very long, just truncate it
		msgSize = MAX_MESSAGE_LENGTH;
	}
	
	// MAX_MESSAGE_LENGTH should fit in uint32_t, please?
	sendRawMessage(client, msg, (uint32_t)msgSize);
}

void clientTick(Client *client) {
	fd_set set;
	FD_ZERO(&set);
	FD_SET(client->socket, &set);
	
	// only check if there is data available, don't wait if no
	struct timeval waitTime = { 0, 0 };
	
	select(client->socket + 1, &set, NULL, NULL, &waitTime);
	
	if (FD_ISSET(client->socket, &set)) {
		readMessage(client);
	}
	
	time_t currentTime = time(NULL);
	if (currentTime > client->lastMessageTime + 15) {
		// last message was a while ago, probably lost connection
		//disconnected(client);
	}
}
