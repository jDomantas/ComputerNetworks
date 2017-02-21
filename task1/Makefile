CC := clang
DISABLED_WARNS := -Wno-missing-noreturn -Wno-missing-prototypes -Wno-format-nonliteral -Wno-padded
COMPILE_OPTIONS := -Weverything --std=c11 $(DISABLED_WARNS) -D_BSD_SOURCE

.PHONY: all clean

all: server client

clean:
	rm server client

server:
	$(CC) $(COMPILE_OPTIONS) $^ -o $@

client:
	$(CC) $(COMPILE_OPTIONS) $^ -o $@ -lncurses

SERVER_FILES := $(wildcard src/server/*.c)
CLIENT_FILES := $(wildcard src/client/*.c)

server: $(SERVER_FILES)
client: $(CLIENT_FILES)
