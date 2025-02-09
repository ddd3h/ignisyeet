CC = gcc
CFLAGS = -Wall -Iinclude
SRCS = src/main.c src/rocket.c src/parameter.c src/output.c
OBJS = $(SRCS:.c=.o)
TARGET = ignisyeet

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $(TARGET) $(OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f src/*.o $(TARGET) output.csv
