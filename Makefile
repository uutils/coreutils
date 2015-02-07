# Config options
ENABLE_LTO     ?= n
ENABLE_STRIP   ?= n

# Binaries
RUSTC          ?= rustc
CARGO          ?= cargo
CC             ?= gcc
RM             := rm

# Install directories
PREFIX         ?= /usr/local
BINDIR         ?= /bin
LIBDIR         ?= /lib

# This won't support any directory with spaces in its name, but you can just
# make a symlink without spaces that points to the directory.
BASEDIR        ?= $(shell pwd)
SRCDIR         := $(BASEDIR)/src
BUILDDIR       := $(BASEDIR)/build
TESTDIR        := $(BASEDIR)/test
TEMPDIR        := $(BASEDIR)/tmp

# Flags
RUSTCFLAGS     := -O
RMFLAGS        :=

RUSTCLIBFLAGS  := $(RUSTCFLAGS) -L $(BUILDDIR)/
RUSTCTESTFLAGS := $(RUSTCFLAGS)

# Handle config setup
ifeq ($(ENABLE_LTO),y)
RUSTCBINFLAGS  := $(RUSTCLIBFLAGS) -Z lto
else
RUSTCBINFLAGS  := $(RUSTCLIBFLAGS)
endif

ifneq ($(ENABLE_STRIP),y)
ENABLE_STRIP   :=
endif

# Possible programs
PROGS       := \
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
  du \
  expand \
  factor \
  false \
  fmt \
  fold \
  link \
  hashsum \
  mkdir \
  mv \
  nl \
  nproc \
  od \
  paste \
  printenv \
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
  touch \
  tr \
  true \
  truncate \
  tsort \
  unexpand \
  unlink \
  uniq \
  wc \
  yes \
  head \
  tail \
  whoami

UNIX_PROGS := \
  chroot \
  groups \
  hostid \
  hostname \
  id \
  kill \
  logname \
  mkfifo \
  nice \
  nohup \
  stdbuf \
  timeout \
  tty \
  uname \
  uptime \
  users

ifneq ($(OS),Windows_NT)
	PROGS    := $(PROGS) $(UNIX_PROGS)
endif

ALIASES := \
	hashsum:md5sum \
	hashsum:sha1sum \
	hashsum:sha224sum \
	hashsum:sha256sum \
	hashsum:sha384sum \
	hashsum:sha512sum

BUILD       ?= $(PROGS)

# Output names
EXES        := \
  $(sort $(filter $(BUILD),$(filter-out $(DONT_BUILD),$(PROGS))))

CRATE_RLIBS :=

INSTALL     ?= $(EXES)

INSTALLEES  := \
  $(filter $(INSTALL),$(filter-out $(DONT_INSTALL),$(EXES) uutils))

# Shared library extension
SYSTEM := $(shell uname)
DYLIB_EXT := 
ifeq ($(SYSTEM),Linux)
	DYLIB_EXT    := so
endif
ifeq ($(SYSTEM),Darwin)
	DYLIB_EXT    := dylib
endif

# Libaries to install
LIBS :=
ifneq (,$(findstring stdbuf, $(INSTALLEES)))
LIBS += libstdbuf.$(DYLIB_EXT)
endif

# Programs with usable tests
TEST_PROGS  := \
  cat \
  cp \
  mkdir \
  mv \
  nl \
  seq \
  sort \
  test \
  tr \
  truncate \
  unexpand

TEST        ?= $(TEST_PROGS)

TESTS       := \
  $(filter $(TEST),$(filter-out $(DONT_TEST),$(filter $(BUILD),$(filter-out $(DONT_BUILD),$(TEST_PROGS)))))

# Setup for building crates
define BUILD_SETUP
X := $(shell $(RUSTC) --print file-names --crate-type rlib $(SRCDIR)/$(1)/$(1).rs)
$(1)_RLIB := $$(X)
CRATE_RLIBS += $$(X)
endef
$(foreach crate,$(EXES),$(eval $(call BUILD_SETUP,$(crate))))

# Utils stuff
EXES_PATHS  := $(addprefix $(BUILDDIR)/,$(EXES))
RLIB_PATHS  := $(addprefix $(BUILDDIR)/,$(CRATE_RLIBS))
command     = sh -c '$(1)'

# Main exe build rule
define EXE_BUILD
$(BUILDDIR)/gen/$(1).rs: $(BUILDDIR)/mkmain
	$(BUILDDIR)/mkmain $(1) $$@

$(BUILDDIR)/$(1): $(BUILDDIR)/gen/$(1).rs $(BUILDDIR)/$($(1)_RLIB) | $(BUILDDIR) deps
	$(RUSTC) $(RUSTCBINFLAGS) --extern test=$(BUILDDIR)/libtest.rlib -o $$@ $$<
	$(if $(ENABLE_STRIP),strip $$@,)
endef

define CRATE_BUILD
-include $(BUILDDIR)/$(1).d

$(BUILDDIR)/$($(1)_RLIB): $(SRCDIR)/$(1)/$(1).rs | $(BUILDDIR) deps
	$(RUSTC) $(RUSTCLIBFLAGS) --extern libc=$(BUILDDIR)/liblibc.rlib --extern time=$(BUILDDIR)/libtime.rlib --extern rand=$(BUILDDIR)/librand.rlib --extern regex=$(BUILDDIR)/libregex.rlib --extern serialize=$(BUILDDIR)/librustc-serialize.rlib --crate-type rlib --emit link,dep-info $$< --out-dir $(BUILDDIR)
endef

# Aliases build rule
ALIAS_SOURCE = $(firstword $(subst :, ,$(1)))
ALIAS_TARGET = $(word 2,$(subst :, ,$(1)))
define MAKE_ALIAS

ifneq ($(ALIAS_TARGET,$(1)),)
all: $(BUILDDIR)/$(call ALIAS_TARGET,$(1))
$(BUILDDIR)/$(call ALIAS_TARGET,$(1)): $(BUILDDIR)/$(call ALIAS_SOURCE,$(1))
	$(call command,install $$@ $$<)
endif

endef

# Test exe built rules
define TEST_BUILD
test_$(1): $(TEMPDIR)/$(1)/$(1)_test $(BUILDDIR)/$(1)
	$(call command,cp $(BUILDDIR)/$(1) $(TEMPDIR)/$(1) && cd $(TEMPDIR)/$(1) && $$<)

$(TEMPDIR)/$(1)/$(1)_test: $(TESTDIR)/$(1).rs | $(TEMPDIR)/$(1)
	$(call command,$(RUSTC) $(RUSTCTESTFLAGS) --extern time=$(BUILDDIR)/libtime.rlib --extern regex=$(BUILDDIR)/libregex.rlib --test -o $$@ $$<)

$(TEMPDIR)/$(1): | $(TEMPDIR)
	$(call command,cp -r $(TESTDIR)/fixtures/$(1) $$@ || mkdir $$@)
endef

# Main rules
all: $(EXES_PATHS) $(BUILDDIR)/uutils

# Creating necessary rules for each targets
$(foreach crate,$(EXES),$(eval $(call CRATE_BUILD,$(crate))))
$(foreach exe,$(EXES),$(eval $(call EXE_BUILD,$(exe))))
$(foreach alias,$(ALIASES),$(eval $(call MAKE_ALIAS,$(alias))))
$(foreach test,$(TESTS),$(eval $(call TEST_BUILD,$(test))))

-include $(BUILDDIR)/uutils.d
$(BUILDDIR)/uutils: $(SRCDIR)/uutils/uutils.rs $(BUILDDIR)/mkuutils $(RLIB_PATHS)
	$(BUILDDIR)/mkuutils $(BUILDDIR)/gen/uutils.rs $(EXES)
	$(RUSTC) $(RUSTCBINFLAGS) --extern test=$(BUILDDIR)/libtest.rlib --emit link,dep-info $(BUILDDIR)/gen/uutils.rs --out-dir $(BUILDDIR)
	$(if $(ENABLE_STRIP),strip $@)
	
# Library for stdbuf
$(BUILDDIR)/libstdbuf.$(DYLIB_EXT): $(SRCDIR)/stdbuf/libstdbuf.rs $(SRCDIR)/stdbuf/libstdbuf.c $(SRCDIR)/stdbuf/libstdbuf.h | $(BUILDDIR)
	cd $(SRCDIR)/stdbuf && \
	$(RUSTC) libstdbuf.rs && \
	$(CC) -c -Wall -Werror -fpic libstdbuf.c -L. -llibstdbuf.a && \
	$(CC) -shared -o libstdbuf.$(DYLIB_EXT) -Wl,--whole-archive liblibstdbuf.a -Wl,--no-whole-archive libstdbuf.o -lpthread && \
	mv *.$(DYLIB_EXT) $(BUILDDIR) && $(RM) *.o && $(RM) *.a
	
$(BUILDDIR)/stdbuf: $(BUILDDIR)/libstdbuf.$(DYLIB_EXT)

# Dependencies
$(BUILDDIR)/.rust-crypto: | $(BUILDDIR)
	cd $(BASEDIR)/deps/rust-crypto && $(CARGO) build --release
	cp -r $(BASEDIR)/deps/rust-crypto/target/release/deps/liblibc*.rlib $(BUILDDIR)/liblibc.rlib
	cp -r $(BASEDIR)/deps/rust-crypto/target/release/deps/librand*.rlib $(BUILDDIR)/librand.rlib
	cp -r $(BASEDIR)/deps/rust-crypto/target/release/deps/librustc-serialize*.rlib $(BUILDDIR)/librustc-serialize.rlib
	cp -r $(BASEDIR)/deps/rust-crypto/target/release/deps/libtime*.rlib $(BUILDDIR)/libtime.rlib
	cp -r $(BASEDIR)/deps/rust-crypto/target/release/libcrypto*.rlib $(BUILDDIR)/libcrypto.rlib
	@touch $@

#$(BUILDDIR)/.rust-time: | $(BUILDDIR)
#	cd $(BASEDIR)/deps/time && $(CARGO) build --release
#	cp -r $(BASEDIR)/deps/time/target/release/libtime*.rlib $(BUILDDIR)/libtime.rlib
#	@touch $@

$(BUILDDIR)/.rust-regex: | $(BUILDDIR)
	cd $(BASEDIR)/deps/regex/regex_macros && $(CARGO) build --release
	cp -r $(BASEDIR)/deps/regex/regex_macros/target/release/libregex_macros* $(BUILDDIR)
	cp -r $(BASEDIR)/deps/regex/regex_macros/target/release/deps/libregex*.rlib $(BUILDDIR)/libregex.rlib
	@touch $@

$(BUILDDIR)/mkmain: mkmain.rs | $(BUILDDIR)
	$(RUSTC) $(RUSTCFLAGS) $< -o $@

$(BUILDDIR)/mkuutils: mkuutils.rs | $(BUILDDIR)
	$(RUSTC) $(RUSTCFLAGS) $< -o $@

$(SRCDIR)/cksum/crc_table.rs: $(SRCDIR)/cksum/gen_table.rs
	cd $(SRCDIR)/cksum && $(RUSTC) $(RUSTCBINFLAGS) gen_table.rs && ./gen_table && $(RM) gen_table

deps: $(BUILDDIR)/.rust-crypto $(BUILDDIR)/.rust-regex $(SRCDIR)/cksum/crc_table.rs

crates:
	echo $(EXES)

test: $(TEMPDIR) $(addprefix test_,$(TESTS))
	$(RM) -rf $(TEMPDIR)

clean:
	$(RM) -rf $(BUILDDIR) $(TEMPDIR) $(BASEDIR)/deps/time/target

$(BUILDDIR):
	git submodule update --init
	mkdir -p $(BUILDDIR)/gen

$(TEMPDIR):
	$(RM) -rf $(TEMPDIR)
	mkdir $(TEMPDIR)

install: $(addprefix $(BUILDDIR)/,$(INSTALLEES))
	mkdir -p $(DESTDIR)$(PREFIX)$(BINDIR)
	for prog in $(INSTALLEES); do \
		install $(BUILDDIR)/$$prog $(DESTDIR)$(PREFIX)$(BINDIR)/$(PROG_PREFIX)$$prog; \
	done
	mkdir -p $(DESTDIR)$(PREFIX)$(LIBDIR)
	for lib in $(LIBS); do \
		install $(BUILDDIR)/$$lib $(DESTDIR)$(PREFIX)$(LIBDIR)/$$lib; \
	done

# TODO: figure out if there is way for prefixes to work with the symlinks
install-multicall: $(BUILDDIR)/uutils
	mkdir -p $(DESTDIR)$(PREFIX)$(BINDIR)
	install $(BUILDDIR)/uutils $(DESTDIR)$(PREFIX)$(BINDIR)/$(PROG_PREFIX)uutils
	cd $(DESTDIR)$(PREFIX)$(BINDIR)
	for prog in $(INSTALLEES); do \
		ln -s $(PROG_PREFIX)uutils $$prog; \
	done
	mkdir -p $(DESTDIR)$(PREFIX)$(LIBDIR)
	for lib in $(LIBS); do \
		install $(BUILDDIR)/$$lib $(DESTDIR)$(PREFIX)$(LIBDIR)/$$lib; \
	done

uninstall:
	rm -f $(addprefix $(DESTDIR)$(PREFIX)$(BINDIR)/$(PROG_PREFIX),$(PROGS))
	rm -f $(addprefix $(DESTDIR)$(PREFIX)$(LIBDIR)/,$(LIBS))

uninstall-multicall:
	rm -f $(addprefix $(DESTDIR)$(PREFIX)$(BINDIR)/,$(PROGS) $(PROG_PREFIX)uutils)
	rm -f $(addprefix $(DESTDIR)$(PREFIX)$(LIBDIR)/,$(LIBS))

# Test under the busybox testsuite
$(BUILDDIR)/busybox: $(BUILDDIR)/uutils
	rm -f $(BUILDDIR)/busybox
	ln -s $(BUILDDIR)/uutils $(BUILDDIR)/busybox

# This is a busybox-specific config file their test suite wants to parse.
$(BUILDDIR)/.config: $(BASEDIR)/.busybox-config $(BUILDDIR)/uutils
	cp $< $@

ifeq ($(BUSYBOX_SRC),)
busytest:
	@echo
	@echo "To run \`busytest\` set BUSYBOX_SRC to the directory of the compiled busybox source code."
	@echo "Optionally set RUNTEST_ARGS to arguments to pass to the busybox \`runtest\` program."
	@echo
	@false
else
busytest: $(BUILDDIR)/busybox $(BUILDDIR)/.config
	(cd $(BUSYBOX_SRC)/testsuite && bindir=$(BUILDDIR) ./runtest $(RUNTEST_ARGS))
endif

.PHONY: $(TEMPDIR) all deps test clean busytest install uninstall
