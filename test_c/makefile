UNAME_S := $(shell uname -s)
CC = gcc
CFLAGS = -Wall -Wextra -fPIC -g -O0 -fvisibility=hidden

ifeq ($(UNAME_S),Linux)
    LIB_NAME = libtestlib.so
    LDFLAGS = -shared -fvisibility=hidden -g
else ifeq ($(UNAME_S),Darwin)
    LIB_NAME = libtestlib.dylib
    LDFLAGS = -dynamiclib -fvisibility=hidden -g
else
    $(error Unsupported platform: $(UNAME_S))
endif

SOURCES = testlib.c
OBJECTS = $(SOURCES:.c=.o)
HEADERS = testlib.h

all: $(LIB_NAME)

$(LIB_NAME): $(OBJECTS)
	$(CC) $(LDFLAGS) -o $@ $^ -lm
%.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJECTS) $(LIB_NAME)

symbols: $(LIB_NAME)
ifeq ($(UNAME_S),Linux)
	@echo "=== exported symbols (t = text/code) ==="
	@nm -D --defined-only $(LIB_NAME) | grep ' T '
	@echo ""
	@echo "=== all symbols (including hidden) ==="
	@nm --defined-only $(LIB_NAME) | grep ' T ' | head -20
else ifeq ($(UNAME_S),Darwin)
	@echo "=== exported symbols ==="
	@nm -g $(LIB_NAME) | grep ' T '
	@echo ""
	@echo "=== all symbols (including hidden) ==="
	@nm $(LIB_NAME) | grep ' T ' | head -20
endif

dwarf: $(LIB_NAME)
ifeq ($(UNAME_S),Linux)
	@echo "=== DWARF debug info summary ==="
	@readelf --debug-dump=info $(LIB_NAME) | head -100
else ifeq ($(UNAME_S),Darwin)
	@echo "=== DWARF debug info summary ==="
	@dwarfdump -v $(LIB_NAME) | head -100
endif

info: $(LIB_NAME)
ifeq ($(UNAME_S),Linux)
	@echo "=== file information ==="
	@file $(LIB_NAME)
	@echo ""
	@echo "=== elf sections ==="
	@readelf -S $(LIB_NAME) | grep -E 'debug|dynsym|symtab'
else ifeq ($(UNAME_S),Darwin)
	@echo "=== file information ==="
	@file $(LIB_NAME)
	@echo ""
	@echo "=== mach-o sections ==="
	@otool -l $(LIB_NAME) | grep -A5 'sectname __debug'
endif

help:
	@echo "available targets:"
	@echo "  all      - build the shared library (default)"
	@echo "  clean    - remove build artifacts"
	@echo "  symbols  - display symbol table"
	@echo "  dwarf    - display dwarf debug information"
	@echo "  info     - display file and section information"
	@echo "  help     - display this help message"
	@echo ""
	@echo "platform detected: $(UNAME_S)"
	@echo "library name: $(LIB_NAME)"

.PHONY: all clean symbols dwarf info help