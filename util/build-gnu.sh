#!/bin/bash
# `build-gnu.bash` ~ builds GNU coreutils (from supplied sources)
#
# UU_MAKE_PROFILE == 'debug' | 'release' ## build profile for *uutils* build; may be supplied by caller, defaults to 'debug'

# spell-checker:ignore (paths) abmon deref discrim eacces getlimits getopt ginstall inacc infloop inotify reflink ; (misc) INT_OFLOW OFLOW baddecode submodules ; (vars/env) SRCDIR

set -e

ME="${0}"
ME_dir="$(dirname -- "$(readlink -fm -- "${ME}")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

echo "ME='${ME}'"
echo "ME_dir='${ME_dir}'"
echo "REPO_main_dir='${REPO_main_dir}'"

### * config (from environment with fallback defaults); note: GNU is expected to be a sibling repo directory

path_UUTILS=${path_UUTILS:-${REPO_main_dir}}
path_GNU="$(readlink -fm -- "${path_GNU:-${path_UUTILS}/../gnu}")"

echo "path_UUTILS='${path_UUTILS}'"
echo "path_GNU='${path_GNU}'"

###

if test ! -d "${path_GNU}"; then
    echo "Could not find GNU (expected at '${path_GNU}')"
    echo "git clone --recurse-submodules https://github.com/coreutils/coreutils.git \"${path_GNU}\""
    exit 1
fi

###

UU_MAKE_PROFILE=${UU_MAKE_PROFILE:-release}
echo "UU_MAKE_PROFILE='${UU_MAKE_PROFILE}'"

UU_BUILD_DIR="${path_UUTILS}/target/${UU_MAKE_PROFILE}"
echo "UU_BUILD_DIR='${UU_BUILD_DIR}'"

cd "${path_UUTILS}" && echo "[ pwd:'${PWD}' ]"
make PROFILE="${UU_MAKE_PROFILE}"
cp "${UU_BUILD_DIR}/install" "${UU_BUILD_DIR}/ginstall" # The GNU tests rename this script before running, to avoid confusion with the make target
# Create *sum binaries
for sum in b2sum b3sum md5sum sha1sum sha224sum sha256sum sha384sum sha512sum; do
    sum_path="${UU_BUILD_DIR}/${sum}"
    test -f "${sum_path}" || cp "${UU_BUILD_DIR}/hashsum" "${sum_path}"
done
test -f "${UU_BUILD_DIR}/[" || cp "${UU_BUILD_DIR}/test" "${UU_BUILD_DIR}/["

##

cd "${path_GNU}" && echo "[ pwd:'${PWD}' ]"

# Any binaries that aren't built become `false` so their tests fail
for binary in $(./build-aux/gen-lists-of-programs.sh --list-progs); do
    bin_path="${UU_BUILD_DIR}/${binary}"
    test -f "${bin_path}" || {
        echo "'${binary}' was not built with uutils, using the 'false' program"
        cp "${UU_BUILD_DIR}/false" "${bin_path}"
    }
done

./bootstrap
./configure --quiet --disable-gcc-warnings
#Add timeout to to protect against hangs
sed -i 's|^"\$@|/usr/bin/timeout 600 "\$@|' build-aux/test-driver
# Change the PATH in the Makefile to test the uutils coreutils instead of the GNU coreutils
sed -i "s/^[[:blank:]]*PATH=.*/  PATH='${UU_BUILD_DIR//\//\\/}\$(PATH_SEPARATOR)'\"\$\$PATH\" \\\/" Makefile
sed -i 's| tr | /usr/bin/tr |' tests/init.sh
make -j "$(nproc)"
# Generate the factor tests, so they can be fixed
# Used to be 36. Reduced to 20 to decrease the log size
for i in {00..20}; do
    make "tests/factor/t${i}.sh"
done

# strip the long stuff
for i in {21..36}; do
    sed -i -e "s/\$(tf)\/t${i}.sh//g" Makefile
done

grep -rl 'path_prepend_' tests/* | xargs sed -i 's| path_prepend_ ./src||'
sed -i -e 's|^seq |/usr/bin/seq |' -e 's|sha1sum |/usr/bin/sha1sum |' tests/factor/t*sh

# Remove tests checking for --version & --help
# Not really interesting for us and logs are too big
sed -i -e '/tests\/misc\/invalid-opt.pl/ D' \
    -e '/tests\/misc\/help-version.sh/ D' \
    -e '/tests\/misc\/help-version-getopt.sh/ D' \
    Makefile

# logs are clotted because of this test
sed -i -e '/tests\/misc\/seq-precision.sh/ D' \
    Makefile

# printf doesn't limit the values used in its arg, so this produced ~2GB of output
sed -i '/INT_OFLOW/ D' tests/misc/printf.sh

# Use the system coreutils where the test fails due to error in a util that is not the one being tested
sed -i 's|stat|/usr/bin/stat|' tests/chgrp/basic.sh tests/cp/existing-perm-dir.sh tests/touch/60-seconds.sh tests/misc/sort-compress-proc.sh
sed -i 's|ls -|/usr/bin/ls -|' tests/chgrp/posix-H.sh tests/chown/deref.sh tests/cp/same-file.sh tests/misc/mknod.sh tests/mv/part-symlink.sh tests/du/8gb.sh
sed -i 's|mkdir |/usr/bin/mkdir |' tests/cp/existing-perm-dir.sh tests/rm/empty-inacc.sh
sed -i 's|timeout \([[:digit:]]\)| /usr/bin/timeout \1|' tests/tail-2/inotify-rotate.sh tests/tail-2/inotify-dir-recreate.sh tests/tail-2/inotify-rotate-resources.sh tests/cp/parent-perm-race.sh tests/ls/infloop.sh tests/misc/sort-exit-early.sh tests/misc/sort-NaN-infloop.sh tests/misc/uniq-perf.sh tests/tail-2/inotify-only-regular.sh tests/tail-2/pipe-f2.sh tests/tail-2/retry.sh tests/tail-2/symlink.sh tests/tail-2/wait.sh tests/tail-2/pid.sh tests/dd/stats.sh tests/tail-2/follow-name.sh tests/misc/shuf.sh # Don't break the function called 'grep_timeout'
sed -i 's|chmod |/usr/bin/chmod |' tests/du/inacc-dir.sh tests/mkdir/p-3.sh tests/tail-2/tail-n0f.sh tests/cp/fail-perm.sh tests/du/inaccessible-cwd.sh tests/mv/i-2.sh tests/chgrp/basic.sh tests/misc/shuf.sh
sed -i 's|sort |/usr/bin/sort |' tests/ls/hyperlink.sh tests/misc/test-N.sh
sed -i 's|split |/usr/bin/split |' tests/misc/factor-parallel.sh
sed -i 's|truncate |/usr/bin/truncate |' tests/split/fail.sh
sed -i 's|dd |/usr/bin/dd |' tests/du/8gb.sh tests/tail-2/big-4gb.sh init.cfg
sed -i 's|id -|/usr/bin/id -|' tests/misc/runcon-no-reorder.sh
sed -i 's|touch |/usr/bin/touch |' tests/cp/preserve-link.sh tests/cp/reflink-perm.sh tests/ls/block-size.sh tests/ls/abmon-align.sh tests/ls/rt-1.sh tests/mv/update.sh tests/misc/ls-time.sh tests/misc/stat-nanoseconds.sh tests/misc/time-style.sh tests/misc/test-N.sh
sed -i 's|ln -|/usr/bin/ln -|' tests/cp/link-deref.sh
sed -i 's|printf |/usr/bin/printf |g' tests/dd/ascii.sh
sed -i 's|cp |/usr/bin/cp |' tests/mv/hard-2.sh
sed -i 's|paste |/usr/bin/paste |' tests/misc/od-endian.sh
sed -i 's|seq |/usr/bin/seq |' tests/misc/sort-discrim.sh

# Add specific timeout to tests that currently hang to limit time spent waiting
sed -i 's|\(^\s*\)seq \$|\1/usr/bin/timeout 0.1 seq \$|' tests/misc/seq-precision.sh tests/misc/seq-long-double.sh

# Remove dup of /usr/bin/ when executed several times
grep -rlE '/usr/bin/\s?/usr/bin' init.cfg tests/* | xargs --no-run-if-empty sed -Ei 's|/usr/bin/\s?/usr/bin/|/usr/bin/|g'

#### Adjust tests to make them work with Rust/coreutils
# in some cases, what we are doing in rust/coreutils is good (or better)
# we should not regress our project just to match what GNU is going.
# So, do some changes on the fly

sed -i -e "s|rm: cannot remove 'e/slink'|rm: cannot remove 'e'|g" tests/rm/fail-eacces.sh

sed -i -e "s|rm: cannot remove 'a/b/file'|rm: cannot remove 'a'|g" tests/rm/cycle.sh

sed -i -e "s|rm: cannot remove directory 'b/a/p'|rm: cannot remove 'b'|g" tests/rm/rm1.sh

sed -i -e "s|rm: cannot remove 'a/1'|rm: cannot remove 'a'|g" tests/rm/rm2.sh

sed -i -e "s|removed directory 'a/'|removed directory 'a'|g" tests/rm/v-slash.sh

test -f "${UU_BUILD_DIR}/getlimits" || cp src/getlimits "${UU_BUILD_DIR}"

# When decoding an invalid base32/64 string, gnu writes everything it was able to decode until
# it hit the decode error, while we don't write anything if the input is invalid.
sed -i "s/\(baddecode.*OUT=>\"\).*\"/\1\"/g" tests/misc/base64.pl
sed -i "s/\(\(b2[ml]_[69]\|b32h_[56]\|z85_8\|z85_35\).*OUT=>\)[^}]*\(.*\)/\1\"\"\3/g" tests/misc/basenc.pl

# add "error: " to the expected error message
sed -i "s/\$prog: invalid input/\$prog: error: invalid input/g" tests/misc/basenc.pl

# basenc: swap out error message for unexpected arg
sed -i "s/  {ERR=>\"\$prog: foobar\\\\n\" \. \$try_help }/  {ERR=>\"error: Found argument '--foobar' which wasn't expected, or isn't valid in this context\n\nUSAGE:\n    basenc [OPTION]... [FILE]\n\nFor more information try --help\n\"}]/" tests/misc/basenc.pl
sed -i "s/  {ERR_SUBST=>\"s\/(unrecognized|unknown) option \[-' \]\*foobar\[' \]\*\/foobar\/\"}],//" tests/misc/basenc.pl

# Remove the check whether a util was built. Otherwise tests against utils like "arch" are not run.
sed -i "s|require_built_ |# require_built_ |g" init.cfg
