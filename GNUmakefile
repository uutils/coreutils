# spell-checker:ignore (misc) testsuite runtest findstring (targets) busytest toybox distclean pkgs nextest ; (vars/env) BINDIR BUILDDIR CARGOFLAGS DESTDIR DOCSDIR INSTALLDIR INSTALLEES MULTICALL DATAROOTDIR TESTDIR manpages

# Config options
ifneq (,$(filter install, $(MAKECMDGOALS)))
 PROFILE?=release
endif
PROFILE         ?= debug
MULTICALL       ?= n
COMPLETIONS     ?= y
MANPAGES        ?= y
LOCALES         ?= y
INSTALL         ?= install

# Needed for the foreach loops to split each loop into a separate command
define newline


endef

PROFILE_CMD := --profile=${PROFILE}
ifeq ($(PROFILE),debug)
	PROFILE_CMD =
endif

# Binaries
CARGO  ?= cargo
CARGOFLAGS ?=

#prefix prepended to all binaries and library dir
PROG_PREFIX ?=

# Install directories
PREFIX ?= /usr/local
DESTDIR ?=
BINDIR ?= $(PREFIX)/bin
DATAROOTDIR ?= $(PREFIX)/share
LIBSTDBUF_DIR ?= $(PREFIX)/libexec/$(PROG_PREFIX)coreutils
# Export variable so that it is used during the build
export LIBSTDBUF_DIR

INSTALLDIR_BIN=$(DESTDIR)$(BINDIR)

# This won't support any directory with spaces in its name, but you can just
# make a symlink without spaces that points to the directory.
BASEDIR       ?= $(shell pwd)
ifdef CARGO_TARGET_DIR
BUILDDIR 	  := $(CARGO_TARGET_DIR)/${PROFILE}
BUILDDIR_UUDOC := $(CARGO_TARGET_DIR)/${PROFILE}
else
BUILDDIR      := $(BASEDIR)/target/$(CARGO_BUILD_TARGET)/${PROFILE}
# uudoc should not be cross build
BUILDDIR_UUDOC := $(BASEDIR)/target/$(PROFILE)
endif
PKG_BUILDDIR  := $(BUILDDIR)/deps
DOCSDIR       := $(BASEDIR)/docs

BUSYBOX_ROOT := $(BASEDIR)/tmp
BUSYBOX_VER  := 1.36.1
BUSYBOX_SRC  := $(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER)

TOYBOX_ROOT := $(BASEDIR)/tmp
TOYBOX_VER  := 0.8.12
TOYBOX_SRC  := $(TOYBOX_ROOT)/toybox-$(TOYBOX_VER)

#------------------------------------------------------------------------
# Detect the host system.
# On Windows uname -s might return MINGW_NT-* or CYGWIN_NT-*.
# Otherwise let it default to the kernel name returned by uname -s
# (Linux, Darwin, FreeBSD, …).
#------------------------------------------------------------------------
OS ?= $(shell uname -s)

# Windows does not allow symlink by default.
# Allow to override LN for AppArmor.
ifneq (,$(findstring _NT,$(OS)))
	LN ?= ln -f
endif
LN ?= ln -sf

# Possible programs
PROGS := \
	$(shell sed -n '/feat_Tier1 = \[/,/\]/p' Cargo.toml | sed '1d;2d' |tr -d '],"\n')\
	$(shell sed -n '/feat_common_core = \[/,/\]/p' Cargo.toml | sed '1d' |tr -d '],"\n')

UNIX_PROGS := \
	$(shell sed -n '/feat_require_unix_core = \[/,/\]/p' Cargo.toml | sed '1d' |tr -d '],"\n') \
	hostid \
	pinky \
	stdbuf \
	uptime \
	users \
	who

SELINUX_PROGS := \
	chcon \
	runcon

$(info Detected OS = $(OS))

ifeq (,$(findstring MINGW,$(OS)))
	PROGS += $(UNIX_PROGS)
endif
ifeq ($(SELINUX_ENABLED),1)
	PROGS += $(SELINUX_PROGS)
endif

UTILS ?= $(filter-out $(SKIP_UTILS),$(PROGS))

ifneq ($(findstring stdbuf,$(UTILS)),)
    # Use external libstdbuf per default. It is more robust than embedding libstdbuf.
	CARGOFLAGS += --features feat_external_libstdbuf
endif

# Programs with usable tests

TESTS       := \
	$(sort $(filter $(UTILS),$(PROGS) $(UNIX_PROGS) $(SELINUX_PROGS)))

TEST_NO_FAIL_FAST :=
TEST_SPEC_FEATURE :=
ifneq ($(SPEC),)
TEST_NO_FAIL_FAST :=--no-fail-fast
TEST_SPEC_FEATURE := test_unimplemented
else ifeq ($(SELINUX_ENABLED),1)
TEST_NO_FAIL_FAST :=
TEST_SPEC_FEATURE := selinux
BUILD_SPEC_FEATURE := selinux
endif

define TEST_BUSYBOX
test_busybox_$(1):
	-(cd $(BUSYBOX_SRC)/testsuite && bindir=$(BUILDDIR) ./runtest $(RUNTEST_ARGS) $(1))
endef

# Output names
EXES        := \
	$(sort $(UTILS))

INSTALLEES  := ${EXES}
ifeq (${MULTICALL}, y)
INSTALLEES  := ${INSTALLEES} coreutils
endif

all: build

build-pkgs:
ifneq (${MULTICALL}, y)
ifdef BUILD_SPEC_FEATURE
	${CARGO} build ${CARGOFLAGS} --features "$(BUILD_SPEC_FEATURE)" ${PROFILE_CMD} $(foreach pkg,$(EXES),-p uu_$(pkg))
else
	${CARGO} build ${CARGOFLAGS} ${PROFILE_CMD} $(foreach pkg,$(EXES),-p uu_$(pkg))
endif
endif

build-coreutils:
	${CARGO} build ${CARGOFLAGS} --features "${EXES} $(BUILD_SPEC_FEATURE)" ${PROFILE_CMD} --no-default-features

ifeq (${MULTICALL}, y)
build: build-coreutils locales
else
build: build-pkgs locales
endif

$(foreach test,$(UTILS),$(eval $(call TEST_BUSYBOX,$(test))))

test:
	${CARGO} test ${CARGOFLAGS} --features "$(TESTS) $(TEST_SPEC_FEATURE)" $(PROFILE_CMD) --no-default-features $(TEST_NO_FAIL_FAST)

nextest:
	${CARGO} nextest run ${CARGOFLAGS} --features "$(TESTS) $(TEST_SPEC_FEATURE)" $(PROFILE_CMD) --no-default-features $(TEST_NO_FAIL_FAST)

test_toybox:
	-(cd $(TOYBOX_SRC)/ && make tests)

toybox-src:
	if [ ! -e "$(TOYBOX_SRC)" ] ; then \
		mkdir -p "$(TOYBOX_ROOT)" ; \
		curl -Ls "https://github.com/landley/toybox/archive/refs/tags/$(TOYBOX_VER).tar.gz" -o "$(TOYBOX_ROOT)/$(TOYBOX_VER).tar.gz" ; \
		tar -C "$(TOYBOX_ROOT)" -xf "$(TOYBOX_ROOT)/$(TOYBOX_VER).tar.gz" ; \
		sed -i -e "s|TESTDIR=\".*\"|TESTDIR=\"$(BUILDDIR)\"|g" $(TOYBOX_SRC)/scripts/test.sh; \
		sed -i -e "s/ || exit 1//g" $(TOYBOX_SRC)/scripts/test.sh; \
	fi ;

busybox-src:
	if [ ! -e "$(BUSYBOX_SRC)" ] ; then \
		mkdir -p "$(BUSYBOX_ROOT)" ; \
		curl -Ls "https://github.com/mirror/busybox/archive/refs/tags/$(subst .,_,$(BUSYBOX_VER)).tar.gz" -o "$(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER).tar.gz" ; \
		tar -C "$(BUSYBOX_ROOT)" -xf "$(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER).tar.gz" ; \
	fi ;

# This is a busybox-specific config file their test suite wants to parse.
$(BUILDDIR)/.config: $(BASEDIR)/.busybox-config
	$(INSTALL) -m 644 $< $@

# Test under the busybox test suite
$(BUILDDIR)/busybox: busybox-src build-coreutils $(BUILDDIR)/.config
	$(INSTALL) -m 755 "$(BUILDDIR)/coreutils" "$(BUILDDIR)/busybox"

prepare-busytest: $(BUILDDIR)/busybox
	# disable inapplicable tests
	-( cd "$(BUSYBOX_SRC)/testsuite" ; if [ -e "busybox.tests" ] ; then mv busybox.tests busybox.tests- ; fi ; )

ifeq ($(EXES),)
busytest:
else
busytest: $(BUILDDIR)/busybox $(addprefix test_busybox_,$(filter-out $(SKIP_UTILS),$(EXES)))
endif

clean:
	cargo clean
	cd $(DOCSDIR) && $(MAKE) clean

distclean: clean
	$(CARGO) clean $(CARGOFLAGS) && $(CARGO) update $(CARGOFLAGS)

ifeq ($(MANPAGES),y)
# Do not cross-build uudoc
build-uudoc:
	@unset CARGO_BUILD_TARGET && ${CARGO} build ${CARGOFLAGS} --bin uudoc --features "uudoc ${EXES}" ${PROFILE_CMD} --no-default-features

install-manpages: build-uudoc
	mkdir -p $(DESTDIR)$(DATAROOTDIR)/man/man1
	$(foreach prog, $(INSTALLEES), \
		$(BUILDDIR_UUDOC)/uudoc manpage $(prog) > $(DESTDIR)$(DATAROOTDIR)/man/man1/$(PROG_PREFIX)$(prog).1 $(newline) \
	)
else
install-manpages:
endif

ifeq ($(COMPLETIONS),y)

install-completions: build-uudoc
	mkdir -p $(DESTDIR)$(DATAROOTDIR)/zsh/site-functions
	mkdir -p $(DESTDIR)$(DATAROOTDIR)/bash-completion/completions
	mkdir -p $(DESTDIR)$(DATAROOTDIR)/fish/vendor_completions.d
	$(foreach prog, $(INSTALLEES), \
		$(BUILDDIR_UUDOC)/uudoc completion $(prog) zsh > $(DESTDIR)$(DATAROOTDIR)/zsh/site-functions/_$(PROG_PREFIX)$(prog) $(newline) \
		$(BUILDDIR_UUDOC)/uudoc completion $(prog) bash > $(DESTDIR)$(DATAROOTDIR)/bash-completion/completions/$(PROG_PREFIX)$(prog).bash $(newline) \
		$(BUILDDIR_UUDOC)/uudoc completion $(prog) fish > $(DESTDIR)$(DATAROOTDIR)/fish/vendor_completions.d/$(PROG_PREFIX)$(prog).fish $(newline) \
	)
else
install-completions:
endif

ifeq ($(LOCALES),y)
locales:
	@# Copy uucore common locales
	@if [ -d "$(BASEDIR)/src/uucore/locales" ]; then \
		mkdir -p "$(BUILDDIR)/locales/uucore"; \
		for locale_file in "$(BASEDIR)"/src/uucore/locales/*.ftl; do \
			$(INSTALL) -m 644 "$$locale_file" "$(BUILDDIR)/locales/uucore/"; \
		done; \
	fi; \
	# Copy utility-specific locales
	@for prog in $(INSTALLEES); do \
		if [ -d "$(BASEDIR)/src/uu/$$prog/locales" ]; then \
			mkdir -p "$(BUILDDIR)/locales/$$prog"; \
			for locale_file in "$(BASEDIR)"/src/uu/$$prog/locales/*.ftl; do \
				if [ "$$(basename "$$locale_file")" != "en-US.ftl" ]; then \
					$(INSTALL) -m 644 "$$locale_file" "$(BUILDDIR)/locales/$$prog/"; \
				fi; \
			done; \
		fi; \
	done

# Some utils require extra locale files outside of their package:
# - *sum binaries need the files from checksum_common
INSTALLEES_WITH_EXTRA_LOCALE = \
	$(INSTALLEES) \
	$(if $(findstring sum, $(INSTALLEES)),checksum_common, )
install-locales:
	@for prog in $(INSTALLEES_WITH_EXTRA_LOCALE); do \
		if [ -d "$(BASEDIR)/src/uu/$$prog/locales" ]; then \
			mkdir -p "$(DESTDIR)$(DATAROOTDIR)/locales/$$prog"; \
			for locale_file in "$(BASEDIR)"/src/uu/$$prog/locales/*.ftl; do \
				if [ "$$(basename "$$locale_file")" != "en-US.ftl" ]; then \
					$(INSTALL) -m 644 "$$locale_file" "$(DESTDIR)$(DATAROOTDIR)/locales/$$prog/"; \
				fi; \
			done; \
		fi; \
	done
else
locales:
install-locales:
endif

install: build install-manpages install-completions install-locales
	mkdir -p $(INSTALLDIR_BIN)
ifneq (,$(and $(findstring stdbuf,$(UTILS)),$(findstring feat_external_libstdbuf,$(CARGOFLAGS))))
	mkdir -p $(DESTDIR)$(LIBSTDBUF_DIR)
ifneq (,$(findstring CYGWIN,$(OS)))
	$(INSTALL) -m 755 $(BUILDDIR)/deps/stdbuf.dll $(DESTDIR)$(LIBSTDBUF_DIR)/libstdbuf.dll
else
	$(INSTALL) -m 755 $(BUILDDIR)/deps/libstdbuf.* $(DESTDIR)$(LIBSTDBUF_DIR)/
endif
endif
ifeq (${MULTICALL}, y)
	$(INSTALL) -m 755 $(BUILDDIR)/coreutils $(INSTALLDIR_BIN)/$(PROG_PREFIX)coreutils
	$(foreach prog, $(filter-out coreutils, $(INSTALLEES)), \
		cd $(INSTALLDIR_BIN) && $(LN) $(PROG_PREFIX)coreutils $(PROG_PREFIX)$(prog) $(newline) \
	)
	$(if $(findstring test,$(INSTALLEES)), cd $(INSTALLDIR_BIN) && $(LN) $(PROG_PREFIX)coreutils $(PROG_PREFIX)[)
else
	$(foreach prog, $(INSTALLEES), \
		$(INSTALL) -m 755 $(BUILDDIR)/$(prog) $(INSTALLDIR_BIN)/$(PROG_PREFIX)$(prog) $(newline) \
	)
	$(if $(findstring test,$(INSTALLEES)), $(INSTALL) -m 755 $(BUILDDIR)/test $(INSTALLDIR_BIN)/$(PROG_PREFIX)[)
endif

uninstall:
ifeq (,$(findstring MINGW,$(OS)))
	rm -f $(DESTDIR)$(LIBSTDBUF_DIR)/libstdbuf.*
	-rm -d $(DESTDIR)$(LIBSTDBUF_DIR) 2>/dev/null || true
endif
ifeq (${MULTICALL}, y)
	rm -f $(addprefix $(INSTALLDIR_BIN)/,$(PROG_PREFIX)coreutils)
endif
	rm -f $(addprefix $(INSTALLDIR_BIN)/$(PROG_PREFIX),$(PROGS))
	rm -f $(INSTALLDIR_BIN)/$(PROG_PREFIX)[
	rm -f $(addprefix $(DESTDIR)$(DATAROOTDIR)/zsh/site-functions/_$(PROG_PREFIX),$(PROGS))
	rm -f $(addprefix $(DESTDIR)$(DATAROOTDIR)/bash-completion/completions/$(PROG_PREFIX),$(PROGS).bash)
	rm -f $(addprefix $(DESTDIR)$(DATAROOTDIR)/fish/vendor_completions.d/$(PROG_PREFIX),$(addsuffix .fish,$(PROGS)))
	rm -f $(addprefix $(DESTDIR)$(DATAROOTDIR)/man/man1/$(PROG_PREFIX),$(addsuffix .1,$(PROGS)))

.PHONY: all build build-coreutils build-pkgs build-uudoc test distclean clean busytest install uninstall
