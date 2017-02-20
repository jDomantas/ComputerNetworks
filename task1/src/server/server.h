#ifndef SERVER_H
#define SERVER_H

#include "network.h"

void onMessage(Server *server, Client *client, const char *msg);
void onConnected(Server *server, Client *client);
void onDisconnected(Server *server, Client *client);

#endif
