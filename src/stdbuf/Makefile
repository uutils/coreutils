BUILDDIR ?= $(OUT_DIR)

CC ?= gcc
RUSTC ?= rustc

# Shared library extension
SYSTEM := $(shell uname)
DYLIB_EXT :=
ifeq ($(SYSTEM),Linux)
	DYLIB_EXT    := so
	DYLIB_FLAGS  := -shared
endif
ifeq ($(SYSTEM),Darwin)
	DYLIB_EXT    := dylib
	DYLIB_FLAGS  := -dynamiclib -undefined dynamic_lookup
endif

all: $(BUILDDIR)/libstdbuf.$(DYLIB_EXT)

$(BUILDDIR):
	mkdir -p $@

$(BUILDDIR)/libstdbuf.$(DYLIB_EXT): build.rs libstdbuf/libstdbuf.rs libstdbuf.c libstdbuf.h | $(BUILDDIR)
	cp $(BUILDDIR)/../../../deps/liblibstdbuf-*.a $(BUILDDIR)/liblibstdbuf.a && \
	$(CC) -c -Wall -Werror -fPIC libstdbuf.c -o $(BUILDDIR)/libstdbuf.o && \
	$(CC) $(DYLIB_FLAGS) -o $@ $(BUILDDIR)/libstdbuf.o -Wl,--whole-archive $(BUILDDIR)/liblibstdbuf.a -Wl,--no-whole-archive

