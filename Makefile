# Config options
PROFILE         ?= debug
ifneq (,$(filter install, $(MAKECMDGOALS)))
override PROFILE:=release
override BUILD:=$(INSTALL)
override DONT_BUILD:=$(DONT_INSTALL)
endif

MULTICALL       ?= n

PROFILE_CMD :=
ifeq ($(PROFILE),release)
	PROFILE_CMD = --release
endif

# Binaries
CARGO  ?= cargo
CARGOFLAGS ?=

# Install directories
DESTDIR ?= /usr/local
BINDIR ?= /bin
LIBDIR ?= /lib

INSTALLDIR_BIN=$(DESTDIR)$(BINDIR)
INSTALLDIR_LIB=$(DESTDIR)$(LIBDIR)

#prefix to apply to uutils binary and all tool binaries
PROG_PREFIX ?=

# This won't support any directory with spaces in its name, but you can just
# make a symlink without spaces that points to the directory.
BASEDIR       ?= $(shell pwd)
BUILDDIR      := $(BASEDIR)/target/${PROFILE}/
PKG_BUILDDIR  := $(BUILDDIR)/deps/

BUSYBOX_ROOT := $(BASEDIR)/tmp/
BUSYBOX_VER := 1.24.1
BUSYBOX_SRC:=$(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER)/

# Possible programs
PROGS       := \
  base64 \
  basename \
  cat \
  cksum \
  comm \
  cp \
  cut \
  dirname \
  echo \
  env \
  expand \
  expr \
  factor \
  false \
  fmt \
  fold \
  link \
  hashsum \
  ln \
  mkdir \
  nl \
  nproc \
  od \
  paste \
  printenv \
  ptx \
  pwd \
  readlink \
  realpath \
  relpath \
  rm \
  rmdir \
  sleep \
  split \
  seq \
  shuf \
  sort \
  sum \
  sync \
  tac \
  tee \
  test \
  tr \
  true \
  truncate \
  tsort \
  unexpand \
  uniq \
  wc \
  yes \
  head \
  tail \
  whoami

UNIX_PROGS := \
  chmod \
  chroot \
  du \
  groups \
  hostid \
  hostname \
  id \
  kill \
  logname \
  mkfifo \
  mv \
  nice \
  nohup \
  stdbuf \
  timeout \
  touch \
  tty \
  uname \
  unlink \
  uptime \
  users

ifneq ($(OS),Windows_NT)
	PROGS    := $(PROGS) $(UNIX_PROGS)
endif

BUILD       ?= $(PROGS)

# Programs with usable tests
TEST_PROGS  := \
	base64 \
	basename \
	cat \
	cksum \
	comm \
	cp \
	cut \
	dirname \
	echo \
	env \
	expr \
	factor \
	false \
	fold \
	hashsum \
	head \
	link \
	ln \
	mkdir \
	mv \
	nl \
	paste \
	ptx \
	pwd \
	readlink \
	realpath \
	rm \
	rmdir \
	seq \
	sort \
	split \
	stdbuf \
	sum \
	tac \
	tail \
	test \
	touch \
	tr \
	true \
	truncate \
	tsort \
	unexpand \
	unlink \
	wc

TEST        ?= $(TEST_PROGS)

TESTS       := \
	$(sort $(filter $(TEST),$(filter-out $(DONT_TEST),$(TEST_PROGS))))

BUSYTEST ?= $(PROGS)
BUSYTESTS       := \
	$(sort $(filter $(BUSYTEST),$(filter-out $(DONT_BUSYTEST),$(PROGS))))


define BUILD_EXE
build_exe_$(1):
	${CARGO} build ${CARGOFLAGS} ${PROFILE_CMD} -p $(1)
endef

define TEST_INTEGRATION
test_integration_$(1): build_exe_$(1)
	${CARGO} test ${CARGOFLAGS} --test $(1) --features $(1) --no-default-features
endef

define TEST_BUSYBOX
test_busybox_$(1): build_exe_$(1)
	(cd $(BUSYBOX_SRC)/testsuite && bindir=$(BUILDDIR) ./runtest $(RUNTEST_ARGS) $(1) )
endef

# Output names
EXES        := \
  $(sort $(filter $(BUILD),$(filter-out $(DONT_BUILD),$(PROGS))))

INSTALL     ?= $(EXES)

INSTALLEES  := \
  $(sort $(filter $(INSTALL),$(filter-out $(DONT_INSTALL),$(EXES) uutils)))

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

# Libaries to install
LIBS :=
ifneq (,$(findstring stdbuf, $(INSTALLEES)))
LIBS += libstdbuf.$(DYLIB_EXT)
endif

all: build

do_install = install ${1}
use_default := 1

$(foreach util,$(EXES),$(eval $(call BUILD_EXE,$(util))))

build-uutils: $(addprefix build_exe_,$(EXES))
	${CARGO} build ${CARGOFLAGS} --features "${EXES}" ${PROFILE_CMD} --no-default-features

build: build-uutils

$(foreach test,$(TESTS),$(eval $(call TEST_INTEGRATION,$(test))))
$(foreach test,$(PROGS),$(eval $(call TEST_BUSYBOX,$(test))))

test: $(addprefix test_integration_,$(TESTS))

busybox-src:
	if [ ! -e $(BUSYBOX_SRC) ]; then \
	mkdir -p $(BUSYBOX_ROOT); \
	wget https://busybox.net/downloads/busybox-$(BUSYBOX_VER).tar.bz2 -P $(BUSYBOX_ROOT); \
	tar -C $(BUSYBOX_ROOT) -xf $(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER).tar.bz2; \
	fi; \

ensure-builddir:
	mkdir -p $(BUILDDIR)

# Test under the busybox testsuite
$(BUILDDIR)/busybox: busybox-src ensure-builddir
	echo -e '#!/bin/bash\n$(PKG_BUILDDIR)./$$1 "$${@:2}"' > $@; \
	chmod +x $@;

# This is a busybox-specific config file their test suite wants to parse.
$(BUILDDIR)/.config: $(BASEDIR)/.busybox-config ensure-builddir
	cp $< $@

ifeq ($(BUSYTESTS),)
busytest:
else
busytest: $(BUILDDIR)/busybox $(BUILDDIR)/.config $(addprefix test_busybox_,$(BUSYTESTS))
endif

clean:
	$(RM) -rf $(BUILDDIR) 

distclean: clean
	$(CARGO) clean $(CARGOFLAGS) && $(CARGO) update $(CARGOFLAGS)

# TODO: figure out if there is way for prefixes to work with the symlinks
install: build 
	mkdir -p $(INSTALLDIR_BIN)
ifeq (${MULTICALL}, y)
	install $(BUILDDIR)/uutils $(INSTALLDIR_BIN)/$(PROG_PREFIX)uutils
	$(foreach prog, $(INSTALLEES), cd $(INSTALLDIR_BIN) && ln -fs $(PROG_PREFIX)uutils $(PROG_PREFIX)$(prog);)
else
	$(foreach prog, $(INSTALLEES), \
		install $(PKG_BUILDDIR)/$(prog) $(INSTALLDIR_BIN)/$(PROG_PREFIX)$(prog);)
endif
	mkdir -p $(INSTALLDIR_LIB)
	$(foreach lib, $(LIBS), install $(BUILDDIR)/$$lib $(INSTALLDIR_LIB)/$(lib);)

uninstall:
ifeq (${MULTICALL}, y)
	rm -f $(addprefix $(INSTALLDIR_BIN)/,$(PROG_PREFIX)uutils)
endif
	rm -f $(addprefix $(INSTALLDIR_BIN)/$(PROG_PREFIX),$(PROGS))
	rm -f $(addprefix $(INSTALLDIR_LIB)/,$(LIBS))

.PHONY: all build test distclean clean busytest install uninstall
