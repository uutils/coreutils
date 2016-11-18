# Config options
PROFILE         ?= debug
MULTICALL       ?= n
INSTALL         ?= install
ifneq (,$(filter install, $(MAKECMDGOALS)))
override PROFILE:=release
endif

PROFILE_CMD :=
ifeq ($(PROFILE),release)
	PROFILE_CMD = --release
endif

RM := rm -rf

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
BUILDDIR      := $(BASEDIR)/target/${PROFILE}
PKG_BUILDDIR  := $(BUILDDIR)/deps

BUSYBOX_ROOT := $(BASEDIR)/tmp
BUSYBOX_VER  := 1.24.1
BUSYBOX_SRC  := $(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER)

# Possible programs
PROGS       := \
  base32 \
  base64 \
  basename \
  cat \
  cksum \
  comm \
  cp \
  cut \
  dircolors \
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
  ls \
  mkdir \
  mktemp \
  more \
  mv \
  nl \
  nproc \
  od \
  paste \
  printenv \
  printf \
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
  arch \
  chgrp \
  chmod \
  chown \
  chroot \
  du \
  groups \
  hostid \
  hostname \
  id \
  install \
  kill \
  logname \
  mkfifo \
  mknod \
  nice \
  nohup \
  pathchk \
  pinky \
  stat \
  stdbuf \
  timeout \
  touch \
  tty \
  uname \
  unlink \
  uptime \
  users \
  who

ifneq ($(OS),Windows_NT)
	PROGS    := $(PROGS) $(UNIX_PROGS)
endif

UTILS ?= $(PROGS)

# Programs with usable tests
TEST_PROGS  := \
	base32 \
	base64 \
	basename \
	cat \
	chgrp \
	chmod \
	chown \
	cksum \
	comm \
	cp \
	cut \
	dircolors \
	dirname \
	echo \
	env \
	expr \
	factor \
	false \
	fold \
	hashsum \
	head \
	install \
	link \
	ln \
	ls \
	mkdir \
	mktemp \
	mv \
	nl \
	od \
	paste \
	pathchk \
	pinky \
	printf \
	ptx \
	pwd \
	readlink \
	realpath \
	rm \
	rmdir \
	seq \
	sort \
	split \
	stat \
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
	uniq \
	unlink \
	wc \
	who

TESTS       := \
	$(sort $(filter $(UTILS),$(filter-out $(SKIP_UTILS),$(TEST_PROGS))))

TEST_NO_FAIL_FAST :=
TEST_SPEC_FEATURE :=
ifneq ($(SPEC),)
TEST_NO_FAIL_FAST :=--no-fail-fast
TEST_SPEC_FEATURE := test_unimplemented
endif

define BUILD_EXE
build_exe_$(1):
	${CARGO} build ${CARGOFLAGS} ${PROFILE_CMD} -p $(1)
endef

define TEST_BUSYBOX
test_busybox_$(1):
	(cd $(BUSYBOX_SRC)/testsuite && bindir=$(BUILDDIR) ./runtest $(RUNTEST_ARGS) $(1) )
endef

# Output names
EXES        := \
  $(sort $(filter $(UTILS),$(filter-out $(SKIP_UTILS),$(PROGS))))

INSTALLEES  := ${EXES}
ifeq (${MULTICALL}, y)
INSTALLEES  := ${INSTALLEES} uutils
endif

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

do_install = $(INSTALL) ${1}
use_default := 1

$(foreach util,$(EXES),$(eval $(call BUILD_EXE,$(util))))

build-pkgs: $(addprefix build_exe_,$(EXES))

build-uutils:
	${CARGO} build ${CARGOFLAGS} --features "${EXES}" ${PROFILE_CMD} --no-default-features

build: build-uutils build-pkgs

$(foreach test,$(filter-out $(SKIP_UTILS),$(PROGS)),$(eval $(call TEST_BUSYBOX,$(test))))

test:
	${CARGO} test ${CARGOFLAGS} --features "$(TESTS) $(TEST_SPEC_FEATURE)" --no-default-features $(TEST_NO_FAIL_FAST)

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
busytest: $(BUILDDIR)/busybox $(addprefix test_busybox_,$(filter-out $(SKIP_UTILS),$(EXES)))
endif

clean:
	$(RM) $(BUILDDIR)

distclean: clean
	$(CARGO) clean $(CARGOFLAGS) && $(CARGO) update $(CARGOFLAGS)

install: build
	mkdir -p $(INSTALLDIR_BIN)
ifeq (${MULTICALL}, y)
	$(INSTALL) $(BUILDDIR)/uutils $(INSTALLDIR_BIN)/$(PROG_PREFIX)uutils
	$(foreach prog, $(filter-out uutils, $(INSTALLEES)), \
		cd $(INSTALLDIR_BIN) && ln -fs $(PROG_PREFIX)uutils $(PROG_PREFIX)$(prog);)
else
	$(foreach prog, $(INSTALLEES), \
		$(INSTALL) $(PKG_BUILDDIR)/$(prog) $(INSTALLDIR_BIN)/$(PROG_PREFIX)$(prog);)
endif
	mkdir -p $(INSTALLDIR_LIB)
	$(foreach lib, $(LIBS), $(INSTALL) $(BUILDDIR)/$(lib) $(INSTALLDIR_LIB)/$(lib);)

uninstall:
ifeq (${MULTICALL}, y)
	rm -f $(addprefix $(INSTALLDIR_BIN)/,$(PROG_PREFIX)uutils)
endif
	rm -f $(addprefix $(INSTALLDIR_BIN)/$(PROG_PREFIX),$(PROGS))
	rm -f $(addprefix $(INSTALLDIR_LIB)/,$(LIBS))

.PHONY: all build test distclean clean busytest install uninstall
