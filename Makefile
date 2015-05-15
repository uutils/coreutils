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
RUSTCBINFLAGS  := $(RUSTCLIBFLAGS) -C lto
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
  basename \
  cat \
  cp \
  env \
  dirname \
  echo \
  factor \
  false \
  fold \
  mkdir \
  mv \
  nl \
  paste \
  pwd \
  readlink \
  seq \
  sort \
  split \
  test \
  tr \
  true \
  truncate \
  tsort \
  unexpand

TEST        ?= $(TEST_PROGS)

TESTS       := \
  $(filter $(TEST),$(filter-out $(DONT_TEST),$(filter $(BUILD),$(filter-out $(DONT_BUILD),$(TEST_PROGS)))))

# figure out what dependencies we need based on which programs we're building
define DEP_INCLUDE
-include $(SRCDIR)/$(1)/deps.mk
endef
# we always depend on libc because common/util does
DEPLIBS := libc
DEPPLUGS :=
# now, add in deps in src/utilname/deps.mk
# if we're testing, only consider the TESTS variable,
# otherwise consider the EXES variable
ifeq ($(MAKECMDGOALS),test)
$(foreach build,$(TESTS),$(eval $(call DEP_INCLUDE,$(build))))
else
$(foreach build,$(sort $(TESTS) $(EXES)),$(eval $(call DEP_INCLUDE,$(build))))
endif
# uniqify deps
DEPLIBS := $(sort $(DEPLIBS))
DEPPLUGS := $(sort $(DEPPLUGS))
# build --extern commandline for rustc
DEP_EXTERN := $(foreach lib,$(subst -,_,$(DEPLIBS)),--extern $(lib)=$(BUILDDIR)/lib$(lib).rlib)
DEP_EXTERN += $(foreach plug,$(subst -,_,$(DEPPLUGS)),--extern $(plug)=$(BUILDDIR)/lib$(plug).$(DYLIB_EXT))

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
RESERVED_EXTERNS := --extern uufalse=$(BUILDDIR)/libfalse.rlib --extern uutrue=$(BUILDDIR)/libtrue.rlib --extern uutest=$(BUILDDIR)/libtest.rlib

# Main exe build rule
define EXE_BUILD
$(BUILDDIR)/gen/$(1).rs: $(BUILDDIR)/mkmain
	$(BUILDDIR)/mkmain $(1) $$@

$(BUILDDIR)/$(1): $(BUILDDIR)/gen/$(1).rs $(BUILDDIR)/$($(1)_RLIB) | $(BUILDDIR) deps
	$(RUSTC) $(RUSTCBINFLAGS) $(RESERVED_EXTERNS) -o $$@ $$<
	$(if $(ENABLE_STRIP),strip $$@,)
endef

# GRRR rust-crypto makes a crate called "crypto".
# This should NOT be allowed by crates.io. GRRRR.
define DEP_BUILD
DEP_$(1):
ifeq ($(1),crypto)
	cd $(BASEDIR)/deps && $(CARGO) build --package rust-crypto --release
else
	cd $(BASEDIR)/deps && $(CARGO) build --package $(1) --release
endif
endef

define CRATE_BUILD
-include $(BUILDDIR)/$(1).d

$(BUILDDIR)/$($(1)_RLIB): $(SRCDIR)/$(1)/$(1).rs | $(BUILDDIR) deps
	$(RUSTC) $(RUSTCLIBFLAGS) $(DEP_EXTERN) --crate-type rlib --emit link,dep-info $$< --out-dir $(BUILDDIR)
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
test_$(1): $(BUILDDIR)/$(1) $(TEMPDIR)/$(1)/$(1)_test
	$(call command,cp $(BUILDDIR)/$(1) $(TEMPDIR)/$(1) && cd $(TEMPDIR)/$(1) && $(TEMPDIR)/$(1)/$(1)_test)

$(TEMPDIR)/$(1)/$(1)_test: $(TESTDIR)/$(1).rs | $(TEMPDIR)/$(1)
	$(call command,$(RUSTC) $(RUSTCTESTFLAGS) $(DEP_EXTERN) --test -o $$@ $$<)

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
$(foreach dep,$(sort $(DEPLIBS) $(DEPPLUGS)),$(eval $(call DEP_BUILD,$(dep))))

-include $(BUILDDIR)/uutils.d
$(BUILDDIR)/uutils: $(SRCDIR)/uutils/uutils.rs $(BUILDDIR)/mkuutils $(RLIB_PATHS)
	$(BUILDDIR)/mkuutils $(BUILDDIR)/gen/uutils.rs $(EXES)
	$(RUSTC) $(RUSTCBINFLAGS) $(RESERVED_EXTERNS) --emit link,dep-info $(BUILDDIR)/gen/uutils.rs --out-dir $(BUILDDIR)
	$(if $(ENABLE_STRIP),strip $@)
	
# Library for stdbuf
$(BUILDDIR)/libstdbuf.$(DYLIB_EXT): $(SRCDIR)/stdbuf/libstdbuf.rs $(SRCDIR)/stdbuf/libstdbuf.c $(SRCDIR)/stdbuf/libstdbuf.h | $(BUILDDIR)
	cd $(SRCDIR)/stdbuf && \
	$(RUSTC) libstdbuf.rs && \
	$(CC) -c -Wall -Werror -fpic libstdbuf.c -L. -llibstdbuf.a && \
	$(CC) -shared -o libstdbuf.$(DYLIB_EXT) -Wl,--whole-archive liblibstdbuf.a -Wl,--no-whole-archive libstdbuf.o -lpthread && \
	mv *.$(DYLIB_EXT) $(BUILDDIR) && $(RM) *.o && $(RM) *.a
	
$(BUILDDIR)/stdbuf: $(BUILDDIR)/libstdbuf.$(DYLIB_EXT)

deps: $(BUILDDIR) $(SRCDIR)/cksum/crc_table.rs $(addprefix DEP_,$(DEPLIBS) $(DEPPLUGS))
	$(foreach lib,$(subst -,_,$(DEPLIBS)),$(shell cp $(BASEDIR)/deps/target/release/deps/lib$(lib)-*.rlib $(BUILDDIR)/lib$(lib).rlib))
	$(foreach plug,$(subst -,_,$(DEPPLUGS)),$(shell cp $(BASEDIR)/deps/target/release/deps/lib$(plug)-*.$(DYLIB_EXT) $(BUILDDIR)/lib$(plug).$(DYLIB_EXT)))

$(BUILDDIR)/mkmain: mkmain.rs | $(BUILDDIR)
	$(RUSTC) $(RUSTCFLAGS) $< -o $@

$(BUILDDIR)/mkuutils: mkuutils.rs | $(BUILDDIR)
	$(RUSTC) $(RUSTCFLAGS) $< -o $@

$(SRCDIR)/cksum/crc_table.rs: $(SRCDIR)/cksum/gen_table.rs
	cd $(SRCDIR)/cksum && $(RUSTC) $(RUSTCBINFLAGS) gen_table.rs && ./gen_table && $(RM) gen_table

$(SRCDIR)/factor/prime_table.rs: $(SRCDIR)/factor/gen_table.rs
	cd $(SRCDIR)/factor && $(RUSTC) $(RUSTCBINFLAGS) gen_table.rs && ./gen_table > $@ && $(RM) gen_table

crates:
	echo $(EXES)

test: $(TEMPDIR) $(addprefix test_,$(TESTS))
	$(RM) -rf $(TEMPDIR)

clean:
	$(RM) -rf $(BUILDDIR) $(TEMPDIR)

distclean: clean
	cd $(BASEDIR)/deps && $(CARGO) clean && $(CARGO) update

$(BUILDDIR):
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

# This rule will build each program, ignore all output, and return pass
# or fail depending on whether the build has errors.
build-check:
	@for prog in $(sort $(PROGS)); do \
		make BUILD="$$prog" >/dev/null 2>&1; status=$$?; \
		if [ $$status -eq 0 ]; \
		then printf "%-10s\t\033[1;32mpass\033[00;m\n" $$prog; \
		else printf "%-10s\t\033[1;31mfail\033[00;m\n" $$prog; \
		fi; \
	done

# This rule will test each program, ignore all output, and return pass
# or fail depending on whether the test has errors.
test-check:
	@for prog in $(sort $(TEST_PROGS)); do \
		make TEST="$$prog" test >/dev/null 2>&1; status=$$?; \
		if [ $$status -eq 0 ]; \
		then printf "%-10s\t\033[1;32mpass\033[00;m\n" $$prog; \
		else printf "%-10s\t\033[1;31mfail\033[00;m\n" $$prog; \
		fi; \
	done

.PHONY: $(TEMPDIR) all deps test distclean clean busytest install uninstall
