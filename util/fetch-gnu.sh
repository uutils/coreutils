#!/bin/bash -e
ver="9.12"
repo=https://github.com/coreutils/coreutils
curl -L "${repo}/releases/download/v${ver}/coreutils-${ver}.tar.xz" | tar --strip-components=1 -xJf -

# Pending upstream merge
curl -L https://raw.githubusercontent.com/oech3/gnu-coreutils/refs/heads/patch-1/tests/dd/nocache_fail.sh > tests/dd/nocache_fail.sh
curl -L https://raw.githubusercontent.com/oech3/gnu-coreutils/refs/heads/no-mtab/tests/df/no-mtab-status.sh > tests/df/no-mtab-status.sh
sed -i '/tests\/df\/no-mtab-status-masked-proc.sh/ D' Makefile
curl -L https://raw.githubusercontent.com/oech3/gnu-coreutils/refs/heads/rm-readdir-fail/tests/rm/rm-readdir-fail.sh > tests/rm/rm-readdir-fail.sh
# TODO stop backporting tests from master at GNU coreutils > $ver
backport=(
  # https://github.com/coreutils/coreutils/issues/355
  env/env.sh
  env/printenv.sh
  # remove LD_PRELOAD
  ls/getxattr-speedup.sh
)
for f in "${backport[@]}"
  do curl -L ${repo}/raw/refs/heads/master/tests/$f > tests/$f
done

# A test that does not exist in $ver at all is absent from its test list, so
# `make check` would silently never run it.  Register those in both the automake
# input and the generated Makefile.in: configure derives Makefile from the
# latter, and build-gnu.sh deliberately keeps automake from re-running.
for f in "${backport[@]}"; do
  grep -qF "tests/$f" tests/local.mk ||
    sed -i "s|^all_tests =.*|&\n  tests/$f\t\t\t\t\\\\|" tests/local.mk
  grep -qF "tests/$f" Makefile.in ||
    sed -i "s|^all_tests =.*|&\n  tests/$f\t\t\t\t\\\\|" Makefile.in
done
