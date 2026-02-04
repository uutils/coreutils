#!/bin/bash -e
ver="9.9"
repo=https://github.com/coreutils/coreutils
curl -L "${repo}/releases/download/v${ver}/coreutils-${ver}.tar.xz" | tar --strip-components=1 -xJf -

# TODO stop backporting tests from master at GNU coreutils > 9.9
curl -L ${repo}/raw/refs/heads/master/tests/timeout/timeout.sh > tests/timeout/timeout.sh
curl -L ${repo}/raw/refs/heads/master/tests/timeout/timeout-group.sh > tests/timeout/timeout-group.sh
curl -L ${repo}/raw/refs/heads/master/tests/mv/hardlink-case.sh > tests/mv/hardlink-case.sh
curl -L ${repo}/raw/refs/heads/master/tests/mkdir/writable-under-readonly.sh > tests/mkdir/writable-under-readonly.sh
curl -L ${repo}/raw/refs/heads/master/tests/cp/cp-mv-enotsup-xattr.sh > tests/cp/cp-mv-enotsup-xattr.sh #spell-checker:disable-line
curl -L ${repo}/raw/refs/heads/master/tests/cp/nfs-removal-race.sh > tests/cp/nfs-removal-race.sh
curl -L ${repo}/raw/refs/heads/master/tests/csplit/csplit-io-err.sh > tests/csplit/csplit-io-err.sh
# Replace tests not compatible with our binaries
sed -i -e 's/no-mtab-status.sh/no-mtab-status-masked-proc.sh/' -e 's/nproc-quota.sh/nproc-quota-systemd.sh/'  tests/local.mk
curl -L ${repo}/raw/refs/heads/master/tests/df/no-mtab-status-masked-proc.sh > tests/df/no-mtab-status-masked-proc.sh
curl -L ${repo}/raw/refs/heads/master/tests/nproc/nproc-quota-systemd.sh > tests/nproc/nproc-quota-systemd.sh
curl -L ${repo}/raw/refs/heads/master/tests/stty/bad-speed.sh > tests/stty/bad-speed.sh
# Better support for single binary
curl -L ${repo}/raw/refs/heads/master/tests/env/env.sh > tests/env/env.sh
# Avoid incorrect PASS
curl -L ${repo}/raw/refs/heads/master/tests/runcon/runcon-compute.sh > tests/runcon/runcon-compute.sh
curl -L ${repo}/raw/refs/heads/master/tests/tac/tac-continue.sh > tests/tac/tac-continue.sh
curl -L ${repo}/raw/refs/heads/master/tests/tail/inotify-dir-recreate.sh > tests/tail/inotify-dir-recreate.sh
# Add tac-continue.sh to root tests (it requires root to mount tmpfs)
# Use sed -i.bak for macOS
sed -i.bak 's|tests/split/l-chunk-root.sh.*|tests/split/l-chunk-root.sh\t\t\t\\\n  tests/tac/tac-continue.sh\t\t\t\\|' tests/local.mk
