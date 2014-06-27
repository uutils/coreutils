include common.mk

PREFIX ?= /usr/local
BINDIR ?= /bin

SRC_DIR=$(shell pwd)

# Possible programs
PROGS       := \
  base64 \
  basename \
  cat \
  cksum \
  comm \
  cp \
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
  rm \
  rmdir \
  sleep \
  seq \
  sum \
  sync \
  tac \
  tee \
  touch \
  tr \
  true \
  truncate \
  unlink \
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

CRATES      := \
  $(sort $(EXES))

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

# Utils stuff
EXES_PATHS  := $(addprefix build/,$(EXES))
command     = sh -c '$(1)'


# Main exe build rule
define EXE_BUILD
build/gen/$(1).rs: build/mkmain
	build/mkmain $(1) build/gen/$(1).rs

build/$(1): build/gen/$(1).rs build/$(1).timestamp | build deps
	$(RUSTC) $(RUSTCFLAGS) -L build/ -o build/$(1) build/gen/$(1).rs
endef

define CRATE_BUILD
-include build/$(1).d
build/$(1).timestamp: $(1)/$(1).rs | build deps
	$(RUSTC) $(RUSTCFLAGS) -L build/ --crate-type rlib --dep-info build/$(1).d $(1)/$(1).rs --out-dir build
	@touch build/$(1).timestamp
endef

# Aliases build rule
ALIAS_SOURCE = $(firstword $(subst :, ,$(1)))
ALIAS_TARGET = $(word 2,$(subst :, ,$(1)))
define MAKE_ALIAS
all: build/$(call ALIAS_TARGET,$(1))
build/$(call ALIAS_TARGET,$(1)): build/$(call ALIAS_SOURCE,$(1))
	$(call command,install build/$(call ALIAS_SOURCE,$(1)) build/$(call ALIAS_TARGET,$(1)))
endef

# Test exe built rules
define TEST_BUILD
test_$(1): tmp/$(1)_test build/$(1)
	$(call command,tmp/$(1)_test)

tmp/$(1)_test: $(1)/test.rs
	$(call command,$(RUSTC) $(RUSTCFLAGS) --test -o tmp/$(1)_test $(1)/test.rs)
endef

# Main rules
all: $(EXES_PATHS) build/uutils

-include build/uutils.d
build/uutils: uutils/uutils.rs $(addprefix build/, $(addsuffix .timestamp, $(CRATES)))
	$(RUSTC) $(RUSTCFLAGS) -L build/ --dep-info $@.d uutils/uutils.rs -o $@

# Dependencies
LIBCRYPTO = $(shell $(RUSTC) --crate-file-name --crate-type rlib deps/rust-crypto/src/rust-crypto/lib.rs)
-include build/rust-crypto.d
build/$(LIBCRYPTO): | build
	$(RUSTC) $(RUSTCFLAGS) --crate-type rlib --dep-info build/rust-crypto.d deps/rust-crypto/src/rust-crypto/lib.rs --out-dir build/

build/mkmain: mkmain.rs | build
	$(RUSTC) $(RUSTCFLAGS) -L build mkmain.rs -o $@

deps: build/$(LIBCRYPTO)

crates:
	echo $(EXES)

test: tmp $(addprefix test_,$(TESTS))
	$(RM) -rf tmp

clean:
	$(RM) -rf build tmp

build:
	git submodule update --init
	mkdir -p build/gen

tmp:
	mkdir tmp

# Creating necessary rules for each targets
$(foreach crate,$(CRATES),$(eval $(call CRATE_BUILD,$(crate))))
$(foreach exe,$(EXES),$(eval $(call EXE_BUILD,$(exe))))
$(foreach alias,$(ALIASES),$(eval $(call MAKE_ALIAS,$(alias))))
$(foreach test,$(TESTS),$(eval $(call TEST_BUILD,$(test))))

install: $(addprefix build/,$(INSTALLEES))
	mkdir -p $(DESTDIR)$(PREFIX)$(BINDIR)
	for prog in $(INSTALLEES); do \
		install build/$$prog $(DESTDIR)$(PREFIX)$(BINDIR)/$(PROG_PREFIX)$$prog; \
	done

uninstall:
	rm -f $(addprefix $(DESTDIR)$(PREFIX)$(BINDIR)/$(PROG_PREFIX),$(PROGS))

# Test under the busybox testsuite
build/busybox: build/uutils
	rm -f build/busybox
	ln -s $(SRC_DIR)/build/uutils build/busybox

# This is a busybox-specific config file their test suite wants to parse.
# For now it's blank.
build/.config: build/uutils
	touch $@

ifeq ($(BUSYBOX_SRC),)
busytest:
	@echo
	@echo "To run \`busytest\` set BUSYBOX_SRC to the directory of the compiled busybox source code."
	@echo "Optionally set RUNTEST_ARGS to arguments to pass to the busybox \`runtest\` program."
	@echo
	@false
else
busytest: build/busybox build/.config
	(cd $(BUSYBOX_SRC)/testsuite && bindir=$(SRC_DIR)/build ./runtest $(RUNTEST_ARGS))
endif

.PHONY: all deps test clean busytest install uninstall
