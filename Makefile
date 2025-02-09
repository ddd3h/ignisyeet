CC = gcc
CFLAGS = -Wall -Iinclude
LDFLAGS = -lm
SRCS = src/main.c src/rocket.c src/parameter.c src/output.c
OBJS = $(SRCS:.c=.o)
TARGET = ignisyeet

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $(TARGET) $(OBJS) $(LDFLAGS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f src/*.o $(TARGET) output.csv