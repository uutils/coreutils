#!/bin/bash
# `build-gnu.bash` ~ builds GNU coreutils (from supplied sources)
#

# spell-checker:ignore (paths) abmon deref discrim eacces getlimits getopt ginstall inacc infloop inotify reflink ; (misc) INT_OFLOW OFLOW baddecode submodules ; (vars/env) SRCDIR vdir rcexp xpart dired

set -e

ME="${0}"
ME_dir="$(dirname -- "$(readlink -fm -- "${ME}")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

# Default profile is 'debug'
UU_MAKE_PROFILE='debug'

for arg in "$@"
do
    if [ "$arg" == "--release-build" ]; then
        UU_MAKE_PROFILE='release'
        break
    fi
done

echo "UU_MAKE_PROFILE='${UU_MAKE_PROFILE}'"

### * config (from environment with fallback defaults); note: GNU is expected to be a sibling repo directory

path_UUTILS=${path_UUTILS:-${REPO_main_dir}}
path_GNU="$(readlink -fm -- "${path_GNU:-${path_UUTILS}/../gnu}")"

###

# On MacOS there is no system /usr/bin/timeout
# and trying to add it to /usr/bin (with symlink of copy binary) will fail unless system integrity protection is disabled (not ideal)
# ref: https://support.apple.com/en-us/102149
# On MacOS the Homebrew coreutils could be installed and then "sudo ln -s /opt/homebrew/bin/timeout /usr/local/bin/timeout"
# Set to /usr/local/bin/timeout instead if /usr/bin/timeout is not found
SYSTEM_TIMEOUT="timeout"
if [ -x /usr/bin/timeout ]; then
    SYSTEM_TIMEOUT="/usr/bin/timeout"
elif [ -x /usr/local/bin/timeout ]; then
    SYSTEM_TIMEOUT="/usr/local/bin/timeout"
fi

###

release_tag_GNU="v9.4"

if test ! -d "${path_GNU}"; then
    echo "Could not find GNU coreutils (expected at '${path_GNU}')"
    echo "Run the following to download into the expected path:"
    echo "git clone --recurse-submodules https://github.com/coreutils/coreutils.git \"${path_GNU}\""
    echo "After downloading GNU coreutils to \"${path_GNU}\" run the following commands to checkout latest release tag"
    echo "cd \"${path_GNU}\""
    echo "git fetch --all --tags"
    echo "git checkout tags/${release_tag_GNU}"
    exit 1
fi

###

echo "ME='${ME}'"
echo "ME_dir='${ME_dir}'"
echo "REPO_main_dir='${REPO_main_dir}'"

echo "path_UUTILS='${path_UUTILS}'"
echo "path_GNU='${path_GNU}'"

###

UU_BUILD_DIR="${path_UUTILS}/target/${UU_MAKE_PROFILE}"
echo "UU_BUILD_DIR='${UU_BUILD_DIR}'"

cd "${path_UUTILS}" && echo "[ pwd:'${PWD}' ]"

if [ "$(uname)" == "Linux" ]; then
    # only set on linux
    export SELINUX_ENABLED=1
fi

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

if test -f gnu-built; then
    # Change the PATH in the Makefile to test the uutils coreutils instead of the GNU coreutils
    sed -i "s/^[[:blank:]]*PATH=.*/  PATH='${UU_BUILD_DIR//\//\\/}\$(PATH_SEPARATOR)'\"\$\$PATH\" \\\/" Makefile
    echo "GNU build already found. Skip"
    echo "'rm -f $(pwd)/gnu-built' to force the build"
    echo "Note: the customization of the tests will still happen"
else
    ./bootstrap --skip-po
    ./configure --quiet --disable-gcc-warnings
    #Add timeout to to protect against hangs
    sed -i 's|^"\$@|'"${SYSTEM_TIMEOUT}"' 600 "\$@|' build-aux/test-driver
    # Change the PATH in the Makefile to test the uutils coreutils instead of the GNU coreutils
    sed -i "s/^[[:blank:]]*PATH=.*/  PATH='${UU_BUILD_DIR//\//\\/}\$(PATH_SEPARATOR)'\"\$\$PATH\" \\\/" Makefile
    sed -i 's| tr | /usr/bin/tr |' tests/init.sh
    make -j "$(nproc)"
    touch gnu-built
fi

# Handle generated factor tests
t_first=00
t_max=36
# t_max_release=20
# if test "${UU_MAKE_PROFILE}" != "debug"; then
#     # Generate the factor tests, so they can be fixed
#     # * reduced to 20 to decrease log size (down from 36 expected by GNU)
#     # * only for 'release', skipped for 'debug' as redundant and too time consuming (causing timeout errors)
#     seq=$(
#         i=${t_first}
#         while test "${i}" -le "${t_max_release}"; do
#             printf '%02d ' $i
#             i=$((i + 1))
#         done
#     )
#     for i in ${seq}; do
#         make "tests/factor/t${i}.sh"
#     done
#     cat
#     sed -i -e 's|^seq |/usr/bin/seq |' -e 's|sha1sum |/usr/bin/sha1sum |' tests/factor/t*.sh
#     t_first=$((t_max_release + 1))
# fi
# strip all (debug) or just the longer (release) factor tests from Makefile
seq=$(
    i=${t_first}
    while test "${i}" -le "${t_max}"; do
        printf '%02d ' ${i}
        i=$((i + 1))
    done
)
for i in ${seq}; do
    echo "strip t${i}.sh from Makefile"
    sed -i -e "s/\$(tf)\/t${i}.sh//g" Makefile
done

grep -rl 'path_prepend_' tests/* | xargs sed -i 's| path_prepend_ ./src||'

# Remove tests checking for --version & --help
# Not really interesting for us and logs are too big
sed -i -e '/tests\/misc\/invalid-opt.pl/ D' \
    -e '/tests\/help\/help-version.sh/ D' \
    -e '/tests\/help\/help-version-getopt.sh/ D' \
    Makefile

# logs are clotted because of this test
sed -i -e '/tests\/seq\/seq-precision.sh/ D' \
    Makefile

# printf doesn't limit the values used in its arg, so this produced ~2GB of output
sed -i '/INT_OFLOW/ D' tests/printf/printf.sh

# Use the system coreutils where the test fails due to error in a util that is not the one being tested
sed -i 's|stat|/usr/bin/stat|' tests/touch/60-seconds.sh tests/sort/sort-compress-proc.sh
sed -i 's|ls -|/usr/bin/ls -|' tests/cp/same-file.sh tests/misc/mknod.sh tests/mv/part-symlink.sh
sed -i 's|chmod |/usr/bin/chmod |' tests/du/inacc-dir.sh tests/tail/tail-n0f.sh tests/cp/fail-perm.sh tests/mv/i-2.sh tests/shuf/shuf.sh
sed -i 's|sort |/usr/bin/sort |' tests/ls/hyperlink.sh tests/test/test-N.sh
sed -i 's|split |/usr/bin/split |' tests/factor/factor-parallel.sh
sed -i 's|id -|/usr/bin/id -|' tests/runcon/runcon-no-reorder.sh
# tests/ls/abmon-align.sh - https://github.com/uutils/coreutils/issues/3505
sed -i 's|touch |/usr/bin/touch |' tests/cp/reflink-perm.sh tests/ls/block-size.sh tests/mv/update.sh tests/ls/ls-time.sh tests/stat/stat-nanoseconds.sh tests/misc/time-style.sh tests/test/test-N.sh tests/ls/abmon-align.sh
sed -i 's|ln -|/usr/bin/ln -|' tests/cp/link-deref.sh
sed -i 's|cp |/usr/bin/cp |' tests/mv/hard-2.sh
sed -i 's|paste |/usr/bin/paste |' tests/od/od-endian.sh
sed -i 's|timeout |'"${SYSTEM_TIMEOUT}"' |' tests/tail/follow-stdin.sh

# Add specific timeout to tests that currently hang to limit time spent waiting
sed -i 's|\(^\s*\)seq \$|\1'"${SYSTEM_TIMEOUT}"' 0.1 seq \$|' tests/seq/seq-precision.sh tests/seq/seq-long-double.sh

# Remove dup of /usr/bin/ and /usr/local/bin/ when executed several times
grep -rlE '/usr/bin/\s?/usr/bin' init.cfg tests/* | xargs --no-run-if-empty sed -Ei 's|/usr/bin/\s?/usr/bin/|/usr/bin/|g'
grep -rlE '/usr/local/bin/\s?/usr/local/bin' init.cfg tests/* | xargs --no-run-if-empty sed -Ei 's|/usr/local/bin/\s?/usr/local/bin/|/usr/local/bin/|g'

#### Adjust tests to make them work with Rust/coreutils
# in some cases, what we are doing in rust/coreutils is good (or better)
# we should not regress our project just to match what GNU is going.
# So, do some changes on the fly

sed -i -e "s|rm: cannot remove 'e/slink'|rm: cannot remove 'e'|g" tests/rm/fail-eacces.sh

sed -i -e "s|rm: cannot remove 'a/b'|rm: cannot remove 'a'|g" tests/rm/fail-2eperm.sh

sed -i -e "s|rm: cannot remove 'a/b/file'|rm: cannot remove 'a'|g" tests/rm/cycle.sh

sed -i -e "s|rm: cannot remove directory 'b/a/p'|rm: cannot remove 'b'|g" tests/rm/rm1.sh

sed -i -e "s|rm: cannot remove 'a/1'|rm: cannot remove 'a'|g" tests/rm/rm2.sh

sed -i -e "s|removed directory 'a/'|removed directory 'a'|g" tests/rm/v-slash.sh

# 'rel' doesn't exist. Our implementation is giving a better message.
sed -i -e "s|rm: cannot remove 'rel': Permission denied|rm: cannot remove 'rel': No such file or directory|g" tests/rm/inaccessible.sh

# overlay-headers.sh test intends to check for inotify events,
# however there's a bug because `---dis` is an alias for: `---disable-inotify`
sed -i -e "s|---dis ||g" tests/tail/overlay-headers.sh

test -f "${UU_BUILD_DIR}/getlimits" || cp src/getlimits "${UU_BUILD_DIR}"

# pr produces very long log and this command isn't super interesting
# SKIP for now
sed -i -e "s|my \$prog = 'pr';$|my \$prog = 'pr';CuSkip::skip \"\$prog: SKIP for producing too long logs\";|" tests/pr/pr-tests.pl

# When decoding an invalid base32/64 string, gnu writes everything it was able to decode until
# it hit the decode error, while we don't write anything if the input is invalid.
sed -i "s/\(baddecode.*OUT=>\"\).*\"/\1\"/g" tests/misc/base64.pl
sed -i "s/\(\(b2[ml]_[69]\|b32h_[56]\|z85_8\|z85_35\).*OUT=>\)[^}]*\(.*\)/\1\"\"\3/g" tests/misc/basenc.pl

# add "error: " to the expected error message
sed -i "s/\$prog: invalid input/\$prog: error: invalid input/g" tests/misc/basenc.pl

# basenc: swap out error message for unexpected arg
sed -i "s/  {ERR=>\"\$prog: foobar\\\\n\" \. \$try_help }/  {ERR=>\"error: Found argument '--foobar' which wasn't expected, or isn't valid in this context\n\n  If you tried to supply '--foobar' as a value rather than a flag, use '-- --foobar'\n\nUsage: basenc [OPTION]... [FILE]\n\nFor more information try '--help'\n\"}]/" tests/misc/basenc.pl
sed -i "s/  {ERR_SUBST=>\"s\/(unrecognized|unknown) option \[-' \]\*foobar\[' \]\*\/foobar\/\"}],//" tests/misc/basenc.pl

# Remove the check whether a util was built. Otherwise tests against utils like "arch" are not run.
sed -i "s|require_built_ |# require_built_ |g" init.cfg
# Some tests are executed with the "nobody" user.
# The check to verify if it works is based on the GNU coreutils version
# making it too restrictive for us
sed -i "s|\$PACKAGE_VERSION|[0-9]*|g" tests/rm/fail-2eperm.sh tests/mv/sticky-to-xpart.sh init.cfg

# usage_vs_getopt.sh is heavily modified as it runs all the binaries
# with the option -/ is used, clap is returning a better error than GNU's. Adjust the GNU test
sed -i -e "s~  grep \" '\*/'\*\" err || framework_failure_~  grep \" '*-/'*\" err || framework_failure_~" tests/misc/usage_vs_getopt.sh
sed -i -e "s~  sed -n \"1s/'\\\/'/'OPT'/p\" < err >> pat || framework_failure_~  sed -n \"1s/'-\\\/'/'OPT'/p\" < err >> pat || framework_failure_~" tests/misc/usage_vs_getopt.sh
# Ignore some binaries (not built)
# And change the default error code to 2
# see issue #3331 (clap limitation).
# Upstream returns 1 for most of the program. We do for cp, truncate & pr
# So, keep it as it
sed -i -e "s/rcexp=1$/rcexp=2\n  case \"\$prg\" in chcon|runcon) return;; esac/" -e "s/rcexp=125 ;;/rcexp=2 ;;\ncp|truncate|pr) rcexp=1;;/" tests/misc/usage_vs_getopt.sh
# GNU has option=[SUFFIX], clap is <SUFFIX>
sed -i -e "s/cat opts/sed -i -e \"s| <.\*>$||g\" opts/" tests/misc/usage_vs_getopt.sh
# for some reasons, some stuff are duplicated, strip that
sed -i -e "s/provoked error./provoked error\ncat pat |sort -u > pat/" tests/misc/usage_vs_getopt.sh

# Update the GNU error message to match ours
sed -i -e "s/link-to-dir: hard link not allowed for directory/failed to create hard link 'link-to-dir' =>/" -e "s|link-to-dir/: hard link not allowed for directory|failed to create hard link 'link-to-dir/' =>|" tests/ln/hard-to-sym.sh

# GNU sleep accepts some crazy string, not sure we should match this behavior
sed -i -e "s/timeout 10 sleep 0x.002p1/#timeout 10 sleep 0x.002p1/" tests/misc/sleep.sh

# install verbose messages shows ginstall as command
sed -i -e "s/ginstall: creating directory/install: creating directory/g" tests/install/basic-1.sh

# GNU doesn't support padding < -LONG_MAX
# disable this test case
sed -i -Ez "s/\n([^\n#]*pad-3\.2[^\n]*)\n([^\n]*)\n([^\n]*)/\n# uutils\/numfmt supports padding = LONG_MIN\n#\1\n#\2\n#\3/" tests/misc/numfmt.pl

# Update the GNU error message to match the one generated by clap
sed -i -e "s/\$prog: multiple field specifications/error: The argument '--field <FIELDS>' was provided more than once, but cannot be used multiple times\n\nUsage: numfmt [OPTION]... [NUMBER]...\n\n\nFor more information try '--help'/g" tests/misc/numfmt.pl
sed -i -e "s/Try 'mv --help' for more information/For more information, try '--help'/g" -e "s/mv: missing file operand/error: the following required arguments were not provided:\n  <files>...\n\nUsage: mv [OPTION]... [-T] SOURCE DEST\n       mv [OPTION]... SOURCE... DIRECTORY\n       mv [OPTION]... -t DIRECTORY SOURCE...\n/g" -e "s/mv: missing destination file operand after 'no-file'/error: The argument '<files>...' requires at least 2 values, but only 1 was provided\n\nUsage: mv [OPTION]... [-T] SOURCE DEST\n       mv [OPTION]... SOURCE... DIRECTORY\n       mv [OPTION]... -t DIRECTORY SOURCE...\n/g" tests/mv/diag.sh

# GNU doesn't support width > INT_MAX
# disable these test cases
sed -i -E "s|^([^#]*2_31.*)$|#\1|g" tests/printf/printf-cov.pl

sed -i -e "s/du: invalid -t argument/du: invalid --threshold argument/" -e "s/du: option requires an argument/error: a value is required for '--threshold <SIZE>' but none was supplied/" -e "/Try 'du --help' for more information./d" tests/du/threshold.sh

awk 'BEGIN {count=0} /compare exp out2/ && count < 6 {sub(/compare exp out2/, "grep -q \"cannot be used with\" out2"); count++} 1' tests/df/df-output.sh > tests/df/df-output.sh.tmp && mv tests/df/df-output.sh.tmp tests/df/df-output.sh

# with ls --dired, in case of error, we have a slightly different error position
sed -i -e "s|44 45|48 49|" tests/ls/stat-failed.sh

# small difference in the error message
sed -i -e "/ls: invalid argument 'XX' for 'time style'/,/Try 'ls --help' for more information\./c\
ls: invalid --time-style argument 'XX'\nPossible values are: [\"full-iso\", \"long-iso\", \"iso\", \"locale\", \"+FORMAT (e.g., +%H:%M) for a 'date'-style format\"]\n\nFor more information try --help" tests/ls/time-style-diag.sh

# disable two kind of tests:
# "hostid BEFORE --help" doesn't fail for GNU. we fail. we are probably doing better
# "hostid BEFORE --help AFTER " same for this
sed -i -e "s/env \$prog \$BEFORE \$opt > out2/env \$prog \$BEFORE \$opt > out2 #/" -e "s/env \$prog \$BEFORE \$opt AFTER > out3/env \$prog \$BEFORE \$opt AFTER > out3 #/" -e "s/compare exp out2/compare exp out2 #/" -e "s/compare exp out3/compare exp out3 #/" tests/help/help-version-getopt.sh
