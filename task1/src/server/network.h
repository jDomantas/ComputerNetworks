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

typedef void (*MessageCallback)(struct Server*, Client*, const char*);
typedef void (*UpdateCallback)(struct Server*, Client*);

typedef struct {
	MessageCallback onMessage;
	UpdateCallback clientConnected;
	UpdateCallback clientDisconnected;
} ServerCallbacks;

typedef struct Server {
	int port;
	int socket;
	struct sockaddr_in address;
	Client clients[MAX_CLIENTS];
	ServerCallbacks callbacks;
} Server;

Server createServer(uint16_t port, ServerCallbacks callbacks);
void sendMessage(Server *server, Client *client, const char *msg);
void sendToAll(Server *server, const char *msg);
void serverTick(Server *server);

#endif
