# Config options
PROFILE         ?= debug
MULTICALL       ?= n

PROFILE_CMD :=
ifeq (${PROFILE},release)
	PROFILE_CMD = --release
endif

# Binaries
CARGO  ?= cargo

# Install directories
PREFIX ?= /usr/local
BINDIR ?= /bin
LIBDIR ?= /lib

INSTALLDIR=$(DESTDIR)$(PREFIX)

# This won't support any directory with spaces in its name, but you can just
# make a symlink without spaces that points to the directory.
BASEDIR       ?= $(shell pwd)
BUILDDIR      := $(BASEDIR)/target/${PROFILE}/
PKG_BUILDDIR  := $(BUILDDIR)/deps/

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

define BUILD_EXE
build_exe_$(1):
	${CARGO} build ${PROFILE_CMD} -p $(1)
endef

define TEST_INTEGRATION
test_integration_$(1): build_exe_$(1)
	${CARGO} test --test $(1) --features $(1) --no-default-features
endef

# Output names
EXES        := \
  $(sort $(filter $(BUILD),$(filter-out $(DONT_BUILD),$(PROGS))))

INSTALLEES  := \
  $(sort $(filter $(INSTALL),$(filter-out $(DONT_INSTALL),$(EXES) uutils)))

INSTALL     ?= $(EXES)

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
	${CARGO} build --features "${EXES}" ${PROFILE_CMD} --no-default-features

build: build-uutils

$(foreach test,$(TESTS),$(eval $(call TEST_INTEGRATION,$(test))))

test: $(addprefix test_integration_,$(TESTS))

clean:
	$(RM) -rf $(BUILDDIR) 

distclean: clean
	$(CARGO) clean && $(CARGO) update

# TODO: figure out if there is way for prefixes to work with the symlinks
install: build
	PROFILE_CMD=--release
	mkdir -p $(INSTALLDIR)$(BINDIR)
ifeq (${MULTICALL}, y)
	install $(BUILDDIR)/uutils $(INSTALLDIR)$(BINDIR)/$(PROG_PREFIX)uutils
	cd $(INSTALLDIR)$(BINDIR)
	$(foreach prog, $(INSTALLEES), ln -s $(PROG_PREFIX)uutils $$prog;)
else
	$(foreach prog, $(INSTALLEES); \
		install $(PKG_BUILDDIR)/$$prog $(INSTALLDIR)$(BINDIR)/$(PROG_PREFIX)$$prog;)
endif
	mkdir -p $(INSTALLDIR)$(LIBDIR)
	$(foreach lib, $(LIBS), install $(BUILDDIR)/$$lib $(INSTALLDIR)$(LIBDIR)/$$lib;)

uninstall:
	rm -f $(addprefix $(INSTALLDIR)$(BINDIR)/$(PROG_PREFIX),$(PROGS))
	rm -f $(addprefix $(INSTALLDIR)$(LIBDIR)/,$(LIBS))

uninstall-multicall:
	rm -f $(addprefix $(INSTALLDIR)$(BINDIR)/,$(PROGS) $(PROG_PREFIX)uutils)
	rm -f $(addprefix $(INSTALLDIR)$(LIBDIR)/,$(LIBS))

.PHONY: all build test distclean clean busytest install uninstall
