#!/bin/bash -e
ver="9.11"
repo=https://github.com/coreutils/coreutils
curl -L "${repo}/releases/download/v${ver}/coreutils-${ver}.tar.xz" | tar --strip-components=1 -xJf -

# TODO stop backporting tests from master at GNU coreutils > $ver
backport=(
  cat/splice.sh # split tests
  dd/fail-ftruncate-fstat.sh # remove LD_PRELOAD
  dd/stderr.sh # replace GNU/test binary by uutils/test
  misc/close-stdout.sh # fix hardcoded pathes to GNU executables
  misc/uname-labeled.sh # uname -A/--all-labeled, added after $ver
  nproc/nproc-quota.sh # remove LD_PRELOAD
  misc/empty-backup-suffix.sh
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
