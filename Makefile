# Config options
PROFILE         ?= debug
MULTICALL       ?= n
ifneq (,$(filter install, $(MAKECMDGOALS)))
override PROFILE:=release
endif

PROFILE_CMD :=
ifeq ($(PROFILE),release)
	PROFILE_CMD = --release
endif

# Binaries
CARGO  ?= cargo
CARGOFLAGS ?=

# Install directories
PREFIX ?= /usr/local
DESTDIR ?= 
BINDIR ?= /bin
LIBDIR ?= /lib

INSTALLDIR_BIN=$(DESTDIR)$(PREFIX)$(BINDIR)
INSTALLDIR_LIB=$(DESTDIR)$(PREFIX)$(LIBDIR)

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
  hashsum \
  head \
  link \
  ln \
  mkdir \
  mktemp \
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
  seq \
  shred \
  shuf \
  sleep \
  sort \
  split \
  sum \
  sync \
  tac \
  tail \
  tee \
  test \
  tr \
  true \
  truncate \
  tsort \
  unexpand \
  uniq \
  wc \
  whoami \
  yes

UNIX_PROGS := \
  chgrp \
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
	chmod \
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
	mktemp \
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

INSTALL  ?= $(PROGS)
BUSYTEST ?= $(PROGS)

ifneq (,$(filter install, $(MAKECMDGOALS)))
override BUILD:=$(INSTALL)
override DONT_BUILD:=$(DONT_INSTALL)
else
ifneq (,$(filter busytest, $(MAKECMDGOALS)))
override BUILD:=$(BUSYTEST)
override DONT_BUILD:=$(DONT_BUSYTEST)
endif
endif

define BUILD_EXE
build_exe_$(1):
	${CARGO} build ${CARGOFLAGS} ${PROFILE_CMD} -p $(1)
endef

define TEST_INTEGRATION
test_integration_$(1): build_exe_$(1)
	${CARGO} test ${CARGOFLAGS} --test $(1) --features $(1) --no-default-features
endef

define TEST_BUSYBOX
test_busybox_$(1):
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

build-pkgs: $(addprefix build_exe_,$(EXES))

build-uutils: 
	${CARGO} build ${CARGOFLAGS} --features "${EXES}" ${PROFILE_CMD} --no-default-features

build: build-uutils build-pkgs

$(foreach test,$(TESTS),$(eval $(call TEST_INTEGRATION,$(test))))
$(foreach test,$(PROGS),$(eval $(call TEST_BUSYBOX,$(test))))

test: $(addprefix test_integration_,$(TESTS))

busybox-src:
	if [ ! -e $(BUSYBOX_SRC) ]; then \
	mkdir -p $(BUSYBOX_ROOT); \
	wget https://busybox.net/downloads/busybox-$(BUSYBOX_VER).tar.bz2 -P $(BUSYBOX_ROOT); \
	tar -C $(BUSYBOX_ROOT) -xf $(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER).tar.bz2; \
	fi; \

# This is a busybox-specific config file their test suite wants to parse.
$(BUILDDIR)/.config: $(BASEDIR)/.busybox-config 
	cp $< $@

# Test under the busybox testsuite
$(BUILDDIR)/busybox: busybox-src build-uutils $(BUILDDIR)/.config 
	cp $(BUILDDIR)/uutils $(BUILDDIR)/busybox; \
	chmod +x $@;

ifeq ($(EXES),)
busytest:
else
busytest: $(BUILDDIR)/busybox $(addprefix test_busybox_,$(EXES))
endif

clean:
	$(RM) -rf $(BUILDDIR) 

distclean: clean
	$(CARGO) clean $(CARGOFLAGS) && $(CARGO) update $(CARGOFLAGS)

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
