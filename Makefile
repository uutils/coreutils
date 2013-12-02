# Binaries
RUSTC       ?= rustc
RM          := rm

# Flags
RUSTCFLAGS  := --opt-level=3
RMFLAGS     :=

# Output names
EXES        := \
  cat \
  echo \
  env \
  false \
  printenv \
  true \
  wc \
  whoami \
  yes \


TESTS       := \
  cat \


# Utils stuff
EXES_PATHS  := $(addprefix build/,$(EXES))
command     = sh -c '$(1)'

# Main exe build rule
define EXE_BUILD
build/$(1): $(1)/$(1).rs
	$(call command,$(RUSTC) $(RUSTCFLAGS) -o build/$(1) $(1)/$(1).rs)
endef

# Test exe built rules
define TEST_BUILD
test_$(1): tmp/$(1)_test build build/$(1)
	$(call command,tmp/$(1)_test)

tmp/$(1)_test: $(1)/test.rs
	$(call command,$(RUSTC) $(RUSTCFLAGS) --test -o tmp/$(1)_test $(1)/test.rs)
endef

# Main rules
all: build $(EXES_PATHS)

test: tmp $(addprefix test_,$(TESTS))
	$(RM) -rf tmp

clean:
	$(RM) -rf build tmp

build:
	mkdir build

tmp:
	mkdir tmp

# Creating necessary rules for each targets
$(foreach exe,$(EXES),$(eval $(call EXE_BUILD,$(exe))))
$(foreach test,$(TESTS),$(eval $(call TEST_BUILD,$(test))))

.PHONY: all test clean
