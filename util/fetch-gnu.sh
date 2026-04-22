#!/bin/bash -e
ver="9.10"
repo=https://github.com/coreutils/coreutils
curl -L "${repo}/releases/download/v${ver}/coreutils-${ver}.tar.xz" | tar --strip-components=1 -xJf -

# TODO stop backporting tests from master at GNU coreutils > $ver
 backport=(
  misc/coreutils.sh # enable test
  tee/tee.sh # input containing sleep
  misc/yes.sh # zero-copy
)
 for f in "${backport[@]}"
  do curl -L ${repo}/raw/refs/heads/master/tests/$f > tests/$f
 done
# adjust for getlimits > $ver
sed -i.b "s/\$ENOSPC/No space left on device/" tests/misc/yes.sh
