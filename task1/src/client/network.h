#ifndef NETWORK_H
#define NETWORK_H

#include <arpa/inet.h>
#include <time.h>

#define MAX_MESSAGE_LENGTH 1000

typedef enum {
	Connected,
	LostConnection,
	Error,
} ClientState;

struct Client;

typedef void (*MessageCallback)(struct Client*, const char*);

typedef struct Client {
	ClientState state;
	time_t lastMessageTime;
	int socket;
	struct sockaddr_in serverAddress;
	size_t nextServerMessageLength;
	MessageCallback callback;
} Client;


Client createClient(const char *address, uint16_t port, MessageCallback callback);
void sendMessage(Client *client, const char *msg);
void clientTick(Client *client);

#endif
