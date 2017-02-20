#ifndef NETWORK_H
#define NETWORK_H

#include <arpa/inet.h>
#include <time.h>

#define MAX_CLIENTS 20
#define MAX_NAME_LENGTH 20
#define MAX_MESSAGE_LENGTH 1000

typedef struct {
	bool isConnected;
	time_t lastPingTime;
	int socket;
	struct sockaddr_in address;
	char name[MAX_NAME_LENGTH + 1];
	size_t nextMessageLength;
} Client;

struct Server;

typedef void (*ClientMessageCallback)(struct Server*, Client*, const char*);

typedef struct Server {
	int port;
	int socket;
	struct sockaddr_in address;
	Client clients[MAX_CLIENTS];
	ClientMessageCallback callback;
} Server;

Server createServer(uint16_t port, ClientMessageCallback callback);
void sendMessage(Server *server, Client *client, const char *msg);
void sendToAll(Server *server, const char *msg);
void serverTick(Server *server);

#endif
