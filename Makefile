include common.mk

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
  dd \
  du \
  factor \
  false \
  fold \
  md5sum \
  mkdir \
  paste \
  printenv \
  pwd \
  rm \
  rmdir \
  sleep \
  seq \
  sum \
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

UNIX_PROGS := \
  hostid \
  hostname \
  kill \
  logname \
  users \
  whoami \
  tty \
  groups \
  id \
  uptime \
  uname

ifneq ($(OS),Windows_NT)
	PROGS    := $(PROGS) $(UNIX_PROGS)
endif

BUILD       ?= $(PROGS)

# Output names
EXES        := \
  $(sort $(filter $(BUILD),$(filter-out $(DONT_BUILD),$(PROGS))))

CRATES      := \
  $(sort $(filter $(EXES), $(filter-out md5sum true false, $(EXES))))

# Programs with usable tests
TEST_PROGS  := \
  cat \
  mkdir \
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
ifeq ($(wildcard $(1)/Makefile),)
build/$(1): $(1)/$(1).rs
	$(call command,$(RUSTC) $(RUSTCFLAGS) -o build/$(1) $(1)/$(1).rs)
clean_$(1):
else
build/$(1): $(1)/$(1).rs
	cd $(1) && make
clean_$(1):
	cd $(1) && make clean
endif
endef

define CRATE_BUILD
build/$(2): $(1)/$(1).rs
	$(call command,$(RUSTC) $(RUSTCFLAGS) --crate-type rlib $(1)/$(1).rs --out-dir build)
endef

# Test exe built rules
define TEST_BUILD
test_$(1): tmp/$(1)_test build build/$(1)
	$(call command,tmp/$(1)_test)

tmp/$(1)_test: $(1)/test.rs
	$(call command,$(RUSTC) $(RUSTCFLAGS) --test -o tmp/$(1)_test $(1)/test.rs)
endef

# Main rules
ifneq ($(MULTICALL), 1)
all: build $(EXES_PATHS)
else
all: build build/uutils

build/uutils: uutils/uutils.rs $(addprefix build/, $(foreach crate,$(CRATES),$(shell $(RUSTC) --crate-type rlib --crate-file-name $(crate)/$(crate).rs)))
	$(RUSTC) $(RUSTCFLAGS) -L build/ uutils/uutils.rs -o $@
endif

test: tmp $(addprefix test_,$(TESTS))
	$(RM) -rf tmp

clean: $(addprefix clean_,$(EXES))
	$(RM) -rf build tmp

build:
	git submodule update --init
	mkdir build

tmp:
	mkdir tmp

# Creating necessary rules for each targets
$(foreach exe,$(EXES),$(eval $(call EXE_BUILD,$(exe))))
$(foreach test,$(TESTS),$(eval $(call TEST_BUILD,$(test))))
ifeq ($(MULTICALL), 1)
$(foreach crate,$(CRATES),$(eval $(call CRATE_BUILD,$(crate),$(shell $(RUSTC) --crate-type rlib --crate-file-name --out-dir build $(crate)/$(crate).rs))))
endif

# Test under the busybox testsuite
ifeq ($(MULTICALL), 1)
build/busybox: build/uutils
	rm -f build/busybox
	ln -s $(SRC_DIR)/build/uutils build/busybox

ifeq ($(BUSYBOX_SRC),)
busytest:
	@echo
	@echo "To run \`busytest\` set BUSYBOX_SRC to the directory of the compiled busybox source code."
	@echo "Optionally set RUNTEST_ARGS to arguments to pass to the busybox \`runtest\` program."
	@echo
	@false
else
busytest: build/busybox
	(cd $(BUSYBOX_SRC)/testsuite && bindir=$(SRC_DIR)/build tstdir=$(BUSYBOX_SRC)/testsuite $(BUSYBOX_SRC)/testsuite/runtest $(RUNTEST_ARGS))
endif
endif

.PHONY: all test clean busytest
