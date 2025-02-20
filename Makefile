CXX = g++
CXXFLAGS = -Wall -std=c++17 -Iinclude
LDFLAGS = -lm
SRCS = src/main.cpp src/rocket.cpp src/parameter.cpp src/output.cpp
OBJS = $(SRCS:.cpp=.o)
TARGET = ignisyeet

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CXX) $(CXXFLAGS) -o $(TARGET) $(OBJS) $(LDFLAGS)

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

clean:
	rm -f src/*.o $(TARGET) output.csv
