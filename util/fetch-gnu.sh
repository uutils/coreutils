#!/bin/bash -e
ver="9.9"
repo=https://github.com/coreutils/coreutils
curl -L "${repo}/releases/download/v${ver}/coreutils-${ver}.tar.xz" | tar --strip-components=1 -xJf -

# TODO stop backporting tests from master at GNU coreutils > 9.9
curl -L ${repo}/raw/refs/heads/master/tests/mv/hardlink-case.sh > tests/mv/hardlink-case.sh
curl -L ${repo}/raw/refs/heads/master/tests/mkdir/writable-under-readonly.sh > tests/mkdir/writable-under-readonly.sh
curl -L ${repo}/raw/refs/heads/master/tests/cp/cp-mv-enotsup-xattr.sh > tests/cp/cp-mv-enotsup-xattr.sh #spell-checker:disable-line
curl -L ${repo}/raw/refs/heads/master/tests/csplit/csplit-io-err.sh > tests/csplit/csplit-io-err.sh
# Replace tests not compatible with our binaries
sed -i -e 's/no-mtab-status.sh/no-mtab-status-masked-proc.sh/' -e 's/nproc-quota.sh/nproc-quota-systemd.sh/'  tests/local.mk
curl -L ${repo}/raw/refs/heads/master/tests/df/no-mtab-status-masked-proc.sh > tests/df/no-mtab-status-masked-proc.sh
curl -L ${repo}/raw/refs/heads/master/tests/nproc/nproc-quota-systemd.sh > tests/nproc/nproc-quota-systemd.sh
# Avoid incorrect PASS
curl -L ${repo}/raw/refs/heads/master/tests/runcon/runcon-compute.sh > tests/runcon/runcon-compute.sh
