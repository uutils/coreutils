#!/bin/bash -e
ver="9.9"
repo=https://github.com/coreutils/coreutils
curl -L "${repo}/releases/download/v${ver}/coreutils-${ver}.tar.xz" | tar --strip-components=1 -xJf -

# backport from coreutils > 9.9
curl ${repo}/raw/refs/heads/master/tests/mv/hardlink-case.sh > tests/mv/hardlink-case.sh
curl ${repo}/raw/refs/heads/master/tests/mkdir/writable-under-readonly.sh > tests/mkdir/writable-under-readonly.sh
curl ${repo}/raw/refs/heads/master/tests/cp/cp-mv-enotsup-xattr.sh > tests/cp/cp-mv-enotsup-xattr.sh #spell-checker:disable-line
