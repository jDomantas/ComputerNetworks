#include <stdio.h>
#include <unistd.h>
#include <string.h>
#include <stdlib.h>
#include "network.h"
#include "reporting.h"
#include "screen.h"

static bool parseLong(const char *input, long *result) {
	char *ep;
	*result = strtol(input, &ep, 0);
	return *ep == 0;
}

static void printUsage(const char *name) {
	printf("usage:\n");
	printf("  %s <server ip> <server port>\n", name);
	exit(1);
}

void onMessage(Client *client, const char *msg) {
	addLine(msg);
}

int main(int argc, char **argv) {
	/*argv[0][0] = (char)argc;
	initScreen();
	//usleep(1000000);
	addLine("Heeeee");
	addLine("Wooo");
	addLine("I am a veeeeeeeeeeeee\x1B[31;1meeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\x1B[0meeeeeeeeery looooooooooong line");
	while (true) {
		screenTick();
	}
	usleep(1000000);
	closeScreen();
	return 0;*/
	
	if (argc != 3) {
		printUsage(argv[0]);
	}
	
	long port;
	if (!parseLong(argv[2], &port) || port < 1 || port > 65535) {
		reportError("Invalid port, must be a number in range 1 - 65535", false);
	}

	Client client = createClient(argv[1], (uint16_t)port, &onMessage);

	initScreen();
	addLine("Connected to server");
	
	while (client.state == Connected) {
		const char *input = getInput();
		if (input != NULL) {
			sendMessage(&client, input);
		}

		clientTick(&client);
	}

	closeScreen();

	if (client.state == LostConnection) {
		puts("Lost connection");
	} else if (client.state == Error) {
		puts("Error occured");
	}

	return 0;
}

