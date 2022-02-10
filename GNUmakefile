# spell-checker:ignore (misc) testsuite runtest findstring (targets) busytest distclean manpages pkgs ; (vars/env) BINDIR BUILDDIR CARGOFLAGS DESTDIR DOCSDIR INSTALLDIR INSTALLEES MANDIR MULTICALL

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
MANDIR ?= /man/man1

INSTALLDIR_BIN=$(DESTDIR)$(PREFIX)$(BINDIR)

#prefix to apply to coreutils binary and all tool binaries
PROG_PREFIX ?=

# This won't support any directory with spaces in its name, but you can just
# make a symlink without spaces that points to the directory.
BASEDIR       ?= $(shell pwd)
BUILDDIR      := $(BASEDIR)/target/${PROFILE}
PKG_BUILDDIR  := $(BUILDDIR)/deps
DOCSDIR       := $(BASEDIR)/docs

BUSYBOX_ROOT := $(BASEDIR)/tmp
BUSYBOX_VER  := 1.32.1
BUSYBOX_SRC  := $(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER)

ifeq ($(SELINUX_ENABLED),)
	SELINUX_ENABLED := 0
	ifneq ($(OS),Windows_NT)
		ifeq ($(shell /sbin/selinuxenabled 2>/dev/null ; echo $$?),0)
			SELINUX_ENABLED := 1
		endif
	endif
endif

# Possible programs
PROGS       := \
	base32 \
	base64 \
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

SELINUX_PROGS := \
	chcon \
	runcon

ifneq ($(OS),Windows_NT)
	PROGS := $(PROGS) $(UNIX_PROGS)
endif

ifeq ($(SELINUX_ENABLED),1)
	PROGS := $(PROGS) $(SELINUX_PROGS)
endif

UTILS ?= $(PROGS)

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
	wc \
	who

TESTS       := \
	$(sort $(filter $(UTILS),$(filter-out $(SKIP_UTILS),$(TEST_PROGS))))

TEST_NO_FAIL_FAST :=
TEST_SPEC_FEATURE :=
ifneq ($(SPEC),)
TEST_NO_FAIL_FAST :=--no-fail-fast
TEST_SPEC_FEATURE := test_unimplemented
else ifeq ($(SELINUX_ENABLED),1)
TEST_NO_FAIL_FAST :=
TEST_SPEC_FEATURE := feat_selinux
endif

define TEST_BUSYBOX
test_busybox_$(1):
	-(cd $(BUSYBOX_SRC)/testsuite && bindir=$(BUILDDIR) ./runtest $(RUNTEST_ARGS) $(1))
endef

# Output names
EXES        := \
	$(sort $(filter $(UTILS),$(filter-out $(SKIP_UTILS),$(PROGS))))

INSTALLEES  := ${EXES}
ifeq (${MULTICALL}, y)
INSTALLEES  := ${INSTALLEES} coreutils
endif

all: build

do_install = $(INSTALL) ${1}
use_default := 1

build-pkgs:
ifneq (${MULTICALL}, y)
	${CARGO} build ${CARGOFLAGS} ${PROFILE_CMD} $(foreach pkg,$(EXES),-p uu_$(pkg))
endif

build-coreutils:
	${CARGO} build ${CARGOFLAGS} --features "${EXES}" ${PROFILE_CMD} --no-default-features

build: build-coreutils build-pkgs

$(foreach test,$(filter-out $(SKIP_UTILS),$(PROGS)),$(eval $(call TEST_BUSYBOX,$(test))))

test:
	${CARGO} test ${CARGOFLAGS} --features "$(TESTS) $(TEST_SPEC_FEATURE)" --no-default-features $(TEST_NO_FAIL_FAST)

busybox-src:
	if [ ! -e "$(BUSYBOX_SRC)" ] ; then \
		mkdir -p "$(BUSYBOX_ROOT)" ; \
		wget "https://busybox.net/downloads/busybox-$(BUSYBOX_VER).tar.bz2" -P "$(BUSYBOX_ROOT)" ; \
		tar -C "$(BUSYBOX_ROOT)" -xf "$(BUSYBOX_ROOT)/busybox-$(BUSYBOX_VER).tar.bz2" ; \
	fi ;

# This is a busybox-specific config file their test suite wants to parse.
$(BUILDDIR)/.config: $(BASEDIR)/.busybox-config
	cp $< $@

# Test under the busybox test suite
$(BUILDDIR)/busybox: busybox-src build-coreutils $(BUILDDIR)/.config
	cp "$(BUILDDIR)/coreutils" "$(BUILDDIR)/busybox"
	chmod +x $@

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

install: build
	mkdir -p $(INSTALLDIR_BIN)
ifeq (${MULTICALL}, y)
	$(INSTALL) $(BUILDDIR)/coreutils $(INSTALLDIR_BIN)/$(PROG_PREFIX)coreutils
	cd $(INSTALLDIR_BIN) && $(foreach prog, $(filter-out coreutils, $(INSTALLEES)), \
		ln -fs $(PROG_PREFIX)coreutils $(PROG_PREFIX)$(prog) &&) :
	$(if $(findstring test,$(INSTALLEES)), cd $(INSTALLDIR_BIN) && ln -fs $(PROG_PREFIX)coreutils $(PROG_PREFIX)[)
else
	$(foreach prog, $(INSTALLEES), \
		$(INSTALL) $(BUILDDIR)/$(prog) $(INSTALLDIR_BIN)/$(PROG_PREFIX)$(prog);)
	$(if $(findstring test,$(INSTALLEES)), $(INSTALL) $(BUILDDIR)/test $(INSTALLDIR_BIN)/$(PROG_PREFIX)[)
endif
	mkdir -p $(DESTDIR)$(PREFIX)/share/zsh/site-functions
	mkdir -p $(DESTDIR)$(PREFIX)/share/bash-completion/completions
	mkdir -p $(DESTDIR)$(PREFIX)/share/fish/vendor_completions.d
	$(foreach prog, $(INSTALLEES), \
		$(BUILDDIR)/coreutils completion $(prog) zsh > $(DESTDIR)$(PREFIX)/share/zsh/site-functions/_$(PROG_PREFIX)$(prog); \
		$(BUILDDIR)/coreutils completion $(prog) bash > $(DESTDIR)$(PREFIX)/share/bash-completion/completions/$(PROG_PREFIX)$(prog); \
		$(BUILDDIR)/coreutils completion $(prog) fish > $(DESTDIR)$(PREFIX)/share/fish/vendor_completions.d/$(PROG_PREFIX)$(prog).fish; \
	)

uninstall:
ifeq (${MULTICALL}, y)
	rm -f $(addprefix $(INSTALLDIR_BIN)/,$(PROG_PREFIX)coreutils)
endif
	rm -f $(addprefix $(INSTALLDIR_BIN)/$(PROG_PREFIX),$(PROGS))
	rm -f $(INSTALLDIR_BIN)/$(PROG_PREFIX)[
	rm -f $(addprefix $(DESTDIR)$(PREFIX)/share/zsh/site-functions/_$(PROG_PREFIX),$(PROGS))
	rm -f $(addprefix $(DESTDIR)$(PREFIX)/share/bash-completion/completions/$(PROG_PREFIX),$(PROGS))
	rm -f $(addprefix $(DESTDIR)$(PREFIX)/share/fish/vendor_completions.d/$(PROG_PREFIX),$(addsuffix .fish,$(PROGS)))

.PHONY: all build build-coreutils build-pkgs test distclean clean busytest install uninstall
