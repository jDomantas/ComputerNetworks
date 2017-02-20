#include <stdio.h>
#include <ncurses.h>
#include <unistd.h>
#include <string.h>
#include <stdlib.h>
#include "network.h"
#include "reporting.h"

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
	printf("Received: %s\n", msg);
}

int main(int argc, char **argv) {
	/*initscr();
	noecho();
	curs_set(FALSE);

	for (int i = 0; i < 5; i++) {
		clear();
		mvprintw(i, 0, "\x1B[31;1m" "Hello, world!" "\x1B[0m");
		refresh();
		usleep(500000);
	}

	endwin();*/
	
	if (argc != 3) {
		printUsage(argv[0]);
	}
	
	long port;
	if (!parseLong(argv[2], &port) || port < 1 || port > 65535) {
		reportError("Invalid port, must be a number in range 1 - 65535", false);
	}

	Client client = createClient(argv[1], (uint16_t)port, &onMessage);

	puts("connected");
	
	sendMessage(&client, "I am a big boy");
	
	sendMessage(&client, "I am a bigger boy");
	
	while (client.state == Connected) {
		clientTick(&client);
	}
	
	if (client.state == LostConnection) {
		puts("Lost connection");
	} else if (client.state == Error) {
		puts("Error occured");
	}
	
	return 0;
}

