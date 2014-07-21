# Config options
ENABLE_LTO    ?= n
ENABLE_STRIP  ?= n

# Binaries
RUSTC         ?= rustc
RM            := rm

# Flags
RUSTCFLAGS    := --opt-level=3
RMFLAGS       :=

# Handle config setup
ifeq ($(ENABLE_LTO),y)
RUSTCBINFLAGS := $(RUSTCFLAGS) -Z lto
else
RUSTCBINFLAGS := $(RUSTCFLAGS)
endif

ifneq ($(ENABLE_STRIP),y)
ENABLE_STRIP :=
endif

# Install directories
PREFIX  ?= /usr/local
BINDIR  ?= /bin

BASEDIR  ?= .
SRCDIR   := $(BASEDIR)/src
BUILDDIR := $(BASEDIR)/build

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
  du \
  factor \
  false \
  fmt \
  fold \
  link \
  hashsum \
  mkdir \
  nl \
  paste \
  printenv \
  pwd \
  realpath \
  relpath \
  rm \
  rmdir \
  sleep \
  split \
  seq \
  shuf \
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
  nohup \
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

# Programs with usable tests
TEST_PROGS  := \
  cat \
  mkdir \
  nl \
  seq \
  tr \
  truncate \

TEST        ?= $(TEST_PROGS)

TESTS       := \
  $(filter $(TEST),$(filter-out $(DONT_TEST),$(filter $(BUILD),$(filter-out $(DONT_BUILD),$(TEST_PROGS)))))

# Setup for building crates
define BUILD_SETUP
X := $(shell $(RUSTC) --print-file-name --crate-type rlib $(SRCDIR)/$(1)/$(1).rs)
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
	$(RUSTC) $(RUSTCBINFLAGS) -L $(BUILDDIR)/ -o $$@ $$<
	$(if $(ENABLE_STRIP),strip $$@,)
endef

define CRATE_BUILD
-include $(BUILDDIR)/$(1).d

$(BUILDDIR)/$($(1)_RLIB): $(SRCDIR)/$(1)/$(1).rs | $(BUILDDIR) deps
	$(RUSTC) $(RUSTCFLAGS) -L $(BUILDDIR)/ --crate-type rlib --dep-info $(BUILDDIR)/$(1).d $$< --out-dir $(BUILDDIR)
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
test_$(1): tmp/$(1)_test $(BUILDDIR)/$(1)
	$(call command,$$<)

tmp/$(1)_test: $(SRCDIR)/$(1)/test.rs
	$(call command,$(RUSTC) $(RUSTCFLAGS) --test -o $$@ $$<)
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
	$(BUILDDIR)/mkuutils $(BUILDDIR)/gen/uutils.rs $(BUILD)
	$(RUSTC) $(RUSTCBINFLAGS) -L $(BUILDDIR)/ --dep-info $@.d $(BUILDDIR)/gen/uutils.rs -o $@
	$(if $(ENABLE_STRIP),strip $@)

# Dependencies
-include $(BUILDDIR)/rust-crypto.d
$(BUILDDIR)/.rust-crypto: | $(BUILDDIR)
	$(RUSTC) $(RUSTCFLAGS) --crate-type rlib --dep-info $(BUILDDIR)/rust-crypto.d $(BASEDIR)/deps/rust-crypto/src/rust-crypto/lib.rs --out-dir $(BUILDDIR)/
	@touch $@

$(BUILDDIR)/mkmain: mkmain.rs | $(BUILDDIR)
	$(RUSTC) $(RUSTCFLAGS) -L $(BUILDDIR) $< -o $@

$(BUILDDIR)/mkuutils: mkuutils.rs | $(BUILDDIR)
	$(RUSTC) $(RUSTCFLAGS) -L $(BUILDDIR) $< -o $@

$(SRCDIR)/cksum/crc_table.rs: $(SRCDIR)/cksum/gen_table.rs
	cd $(SRCDIR)/cksum && $(RUSTC) $(RUSTCFLAGS) gen_table.rs && ./gen_table && $(RM) gen_table

deps: $(BUILDDIR)/.rust-crypto $(SRCDIR)/cksum/crc_table.rs

crates:
	echo $(EXES)

test: tmp $(addprefix test_,$(TESTS))
	$(RM) -rf tmp

clean:
	$(RM) -rf $(BUILDDIR) tmp

$(BUILDDIR):
	git submodule update --init
	mkdir -p $(BUILDDIR)/gen

tmp:
	mkdir tmp

install: $(addprefix $(BUILDDIR)/,$(INSTALLEES))
	mkdir -p $(DESTDIR)$(PREFIX)$(BINDIR)
	for prog in $(INSTALLEES); do \
		install $(BUILDDIR)/$$prog $(DESTDIR)$(PREFIX)$(BINDIR)/$(PROG_PREFIX)$$prog; \
	done

# TODO: figure out if there is way for prefixes to work with the symlinks
install-multicall: $(BUILDDIR)/uutils
	mkdir -p $(DESTDIR)$(PREFIX)$(BINDIR)
	install $(BUILDDIR)/uutils $(DESTDIR)$(PREFIX)$(BINDIR)/$(PROG_PREFIX)uutils
	cd $(DESTDIR)$(PREFIX)$(BINDIR)
	for prog in $(INSTALLEES); do \
		ln -s $(PROG_PREFIX)uutils $$prog; \
	done

uninstall:
	rm -f $(addprefix $(DESTDIR)$(PREFIX)$(BINDIR)/$(PROG_PREFIX),$(PROGS))

uninstall-multicall:
	rm -f $(addprefix $(DESTDIR)$(PREFIX)$(BINDIR)/,$(PROGS) $(PROG_PREFIX)uutils)

# Test under the busybox testsuite
$(BUILDDIR)/busybox: $(BUILDDIR)/uutils
	rm -f $(BUILDDIR)/busybox
	ln -s $(BUILDDIR)/uutils $(BUILDDIR)/busybox

# This is a busybox-specific config file their test suite wants to parse.
# For now it's blank.
$(BUILDDIR)/.config: $(BUILDDIR)/uutils
	touch $@

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

.PHONY: all deps test clean busytest install uninstall
