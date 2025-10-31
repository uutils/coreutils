# spell-checker:ignore (misc) testsuite runtest findstring (targets) busytest toybox distclean pkgs nextest ; (vars/env) BINDIR BUILDDIR CARGOFLAGS DESTDIR DOCSDIR INSTALLDIR INSTALLEES MULTICALL DATAROOTDIR TESTDIR manpages

# Config options
PROFILE         ?= debug
MULTICALL       ?= n
COMPLETIONS     ?= y
MANPAGES        ?= y
LOCALES         ?= y
INSTALL         ?= install
ifneq (,$(filter install, $(MAKECMDGOALS)))
override PROFILE:=release
endif

# Needed for the foreach loops to split each loop into a separate command
define newline


endef

PROFILE_CMD :=
ifeq ($(PROFILE),release)
	PROFILE_CMD = --release
endif

# Binaries
CARGO  ?= cargo
CARGOFLAGS ?=
RUSTC_ARCH ?= # should be empty except for cross-build, not --target $(shell rustc -vV | sed -n 's/host: //p')

# Install directories
PREFIX ?= /usr/local
DESTDIR ?=
BINDIR ?= $(PREFIX)/bin
DATAROOTDIR ?= $(PREFIX)/share
LIBSTDBUF_DIR ?= $(PREFIX)/libexec/coreutils
# Export variable so that it is used during the build
export LIBSTDBUF_DIR

INSTALLDIR_BIN=$(DESTDIR)$(BINDIR)

#prefix to apply to coreutils binary and all tool binaries
PROG_PREFIX ?=

# This won't support any directory with spaces in its name, but you can just
# make a symlink without spaces that points to the directory.
BASEDIR       ?= $(shell pwd)
ifdef CARGO_TARGET_DIR
BUILDDIR 	  := $(CARGO_TARGET_DIR)/${PROFILE}
else
BUILDDIR      := $(BASEDIR)/target/${PROFILE}
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
# On Windows the environment already sets  OS = Windows_NT.
# Otherwise let it default to the kernel name returned by uname -s
# (Linux, Darwin, FreeBSD, …).
#------------------------------------------------------------------------
OS ?= $(shell uname -s)

# Windows does not allow symlink by default.
# Allow to override LN for AppArmor.
ifeq ($(OS),Windows_NT)
	LN ?= ln -f
endif
LN ?= ln -sf

ifdef SELINUX_ENABLED
	override SELINUX_ENABLED := 0
# Now check if we should enable it (only on non-Windows)
	ifneq ($(OS),Windows_NT)
		ifeq ($(shell if [ -x /sbin/selinuxenabled ] && /sbin/selinuxenabled 2>/dev/null; then echo 0; else echo 1; fi),0)
			override SELINUX_ENABLED := 1
$(info /sbin/selinuxenabled successful)
	    else
$(info SELINUX_ENABLED=1 but /sbin/selinuxenabled failed)
		endif
	endif
endif

# Possible programs
PROGS       := \
	arch \
	base32 \
	base64 \
	basenc \
	basename \
	cat \
	cksum \
	comm \
	cp \
	csplit \
	cut \
	date \
	dd \
	df \
	dir \
	dircolors \
	dirname \
	du \
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
	hostname \
	join \
	link \
	ln \
	ls \
	mkdir \
	mktemp \
	more \
	mv \
	nl \
	numfmt \
	nproc \
	od \
	paste \
	pr \
	printenv \
	printf \
	ptx \
	pwd \
	readlink \
	realpath \
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
	touch \
	tr \
	true \
	truncate \
	tsort \
	uname \
	unexpand \
	uniq \
	unlink \
	vdir \
	wc \
	whoami \
	yes

UNIX_PROGS := \
	chgrp \
	chmod \
	chown \
	chroot \
	groups \
	hostid \
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
	stty \
	timeout \
	tty \
	uptime \
	users \
	who

SELINUX_PROGS := \
	chcon \
	runcon

HASHSUM_PROGS := \
	b2sum \
	b3sum \
	md5sum \
	sha1sum \
	sha224sum \
	sha256sum \
	sha3-224sum \
	sha3-256sum \
	sha3-384sum \
	sha3-512sum \
	sha384sum \
	sha3sum \
	sha512sum \
	shake128sum \
	shake256sum

$(info Detected OS = $(OS))

# Don't build the SELinux programs on macOS (Darwin) and FreeBSD
ifeq ($(filter $(OS),Darwin FreeBSD),$(OS))
	SELINUX_PROGS :=
endif

ifneq ($(OS),Windows_NT)
	PROGS := $(PROGS) $(UNIX_PROGS)
# Build the selinux command even if not on the system
	PROGS := $(PROGS) $(SELINUX_PROGS)
endif

UTILS ?= $(filter-out $(SKIP_UTILS),$(PROGS))
ifneq ($(filter hashsum,$(UTILS)),hashsum)
	HASHSUM_PROGS :=
endif

ifneq ($(findstring stdbuf,$(UTILS)),)
    # Use external libstdbuf per default. It is more robust than embedding libstdbuf.
	CARGOFLAGS += --features feat_external_libstdbuf
endif

# Programs with usable tests
TEST_PROGS  := \
	base32 \
	base64 \
	basename \
	cat \
	chcon \
	chgrp \
	chmod \
	chown \
	cksum \
	comm \
	cp \
	csplit \
	cut \
	date \
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
	numfmt \
	od \
	paste \
	pathchk \
	pinky \
	pr \
	printf \
	ptx \
	pwd \
	readlink \
	realpath \
	rm \
	rmdir \
	runcon \
	seq \
	sleep \
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
	uname \
	unexpand \
	uniq \
	unlink \
	uudoc \
	wc \
	who

TESTS       := \
	$(sort $(filter $(UTILS),$(TEST_PROGS)))

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

use_default := 1

build-pkgs:
ifneq (${MULTICALL}, y)
ifdef BUILD_SPEC_FEATURE
	${CARGO} build ${CARGOFLAGS} --features "$(BUILD_SPEC_FEATURE)" ${PROFILE_CMD} $(foreach pkg,$(EXES),-p uu_$(pkg)) $(RUSTC_ARCH)
else
	${CARGO} build ${CARGOFLAGS} ${PROFILE_CMD} $(foreach pkg,$(EXES),-p uu_$(pkg)) $(RUSTC_ARCH)
endif
endif

build-coreutils:
	${CARGO} build ${CARGOFLAGS} --features "${EXES} $(BUILD_SPEC_FEATURE)" ${PROFILE_CMD} --no-default-features $(RUSTC_ARCH)

build: build-coreutils build-pkgs locales

$(foreach test,$(UTILS),$(eval $(call TEST_BUSYBOX,$(test))))

test:
	${CARGO} test ${CARGOFLAGS} --features "$(TESTS) $(TEST_SPEC_FEATURE)" --no-default-features $(TEST_NO_FAIL_FAST)

nextest:
	${CARGO} nextest run ${CARGOFLAGS} --features "$(TESTS) $(TEST_SPEC_FEATURE)" --no-default-features $(TEST_NO_FAIL_FAST)

test_toybox:
	-(cd $(TOYBOX_SRC)/ && make tests)

toybox-src:
	if [ ! -e "$(TOYBOX_SRC)" ] ; then \
		mkdir -p "$(TOYBOX_ROOT)" ; \
		wget "https://github.com/landley/toybox/archive/refs/tags/$(TOYBOX_VER).tar.gz" -P "$(TOYBOX_ROOT)" ; \
		tar -C "$(TOYBOX_ROOT)" -xf "$(TOYBOX_ROOT)/$(TOYBOX_VER).tar.gz" ; \
		sed -i -e "s|TESTDIR=\".*\"|TESTDIR=\"$(BUILDDIR)\"|g" $(TOYBOX_SRC)/scripts/test.sh; \
		sed -i -e "s/ || exit 1//g" $(TOYBOX_SRC)/scripts/test.sh; \
	fi ;

busybox-src:
	if [ ! -e "$(BUSYBOX_SRC)" ] ; then \
		mkdir -p "$(BUSYBOX_ROOT)" ; \
		wget "https://busybox.net/downloads/busybox-$(BUSYBOX_VER).tar.bz2" -P "$(BUSYBOX_ROOT)" ; \
		tar -C "$(BUSYBOX_ROOT)" -xf "$(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER).tar.bz2" ; \
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
	cargo clean $(RUSTC_ARCH)
	cd $(DOCSDIR) && $(MAKE) clean $(RUSTC_ARCH)

distclean: clean
	$(CARGO) clean $(CARGOFLAGS) $(RUSTC_ARCH) && $(CARGO) update $(CARGOFLAGS) $(RUSTC_ARCH)

ifeq ($(MANPAGES),y)
build-uudoc:
	# Use same PROFILE with coreutils to share crates (if not cross-build)
	${CARGO} build ${CARGOFLAGS} --bin uudoc --features "uudoc ${EXES}" ${PROFILE_CMD} --no-default-features

install-manpages: build-uudoc
	mkdir -p $(DESTDIR)$(DATAROOTDIR)/man/man1
	$(foreach prog, $(INSTALLEES) $(HASHSUM_PROGS), \
		$(BUILDDIR)/uudoc manpage $(prog) > $(DESTDIR)$(DATAROOTDIR)/man/man1/$(PROG_PREFIX)$(prog).1 $(newline) \
	)
else
install-manpages:
endif

ifeq ($(COMPLETIONS),y)

install-completions: build-uudoc
	mkdir -p $(DESTDIR)$(DATAROOTDIR)/zsh/site-functions
	mkdir -p $(DESTDIR)$(DATAROOTDIR)/bash-completion/completions
	mkdir -p $(DESTDIR)$(DATAROOTDIR)/fish/vendor_completions.d
	$(foreach prog, $(INSTALLEES) $(HASHSUM_PROGS) , \
		$(BUILDDIR)/uudoc completion $(prog) zsh > $(DESTDIR)$(DATAROOTDIR)/zsh/site-functions/_$(PROG_PREFIX)$(prog) $(newline) \
		$(BUILDDIR)/uudoc completion $(prog) bash > $(DESTDIR)$(DATAROOTDIR)/bash-completion/completions/$(PROG_PREFIX)$(prog).bash $(newline) \
		$(BUILDDIR)/uudoc completion $(prog) fish > $(DESTDIR)$(DATAROOTDIR)/fish/vendor_completions.d/$(PROG_PREFIX)$(prog).fish $(newline) \
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


install-locales:
	@for prog in $(INSTALLEES); do \
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
install-locales:
endif

install: build install-manpages install-completions install-locales
	mkdir -p $(INSTALLDIR_BIN)
ifneq (,$(and $(findstring stdbuf,$(UTILS)),$(findstring feat_external_libstdbuf,$(CARGOFLAGS))))
	mkdir -p $(DESTDIR)$(LIBSTDBUF_DIR)
	$(INSTALL) -m 755 $(BUILDDIR)/deps/libstdbuf* $(DESTDIR)$(LIBSTDBUF_DIR)/
endif
ifeq (${MULTICALL}, y)
	$(INSTALL) -m 755 $(BUILDDIR)/coreutils $(INSTALLDIR_BIN)/$(PROG_PREFIX)coreutils
	$(foreach prog, $(filter-out coreutils, $(INSTALLEES)), \
		cd $(INSTALLDIR_BIN) && $(LN) $(PROG_PREFIX)coreutils $(PROG_PREFIX)$(prog) $(newline) \
	)
	$(foreach prog, $(HASHSUM_PROGS), \
		cd $(INSTALLDIR_BIN) && $(LN) $(PROG_PREFIX)coreutils $(PROG_PREFIX)$(prog) $(newline) \
	)
	$(if $(findstring test,$(INSTALLEES)), cd $(INSTALLDIR_BIN) && $(LN) $(PROG_PREFIX)coreutils $(PROG_PREFIX)[)
else
	$(foreach prog, $(INSTALLEES), \
		$(INSTALL) -m 755 $(BUILDDIR)/$(prog) $(INSTALLDIR_BIN)/$(PROG_PREFIX)$(prog) $(newline) \
	)
	$(foreach prog, $(HASHSUM_PROGS), \
		cd $(INSTALLDIR_BIN) && $(LN) $(PROG_PREFIX)hashsum $(PROG_PREFIX)$(prog) $(newline) \
	)
	$(if $(findstring test,$(INSTALLEES)), $(INSTALL) -m 755 $(BUILDDIR)/test $(INSTALLDIR_BIN)/$(PROG_PREFIX)[)
endif

uninstall:
ifneq ($(OS),Windows_NT)
	rm -f $(DESTDIR)$(LIBSTDBUF_DIR)/libstdbuf*
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
