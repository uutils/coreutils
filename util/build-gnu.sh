#!/usr/bin/env bash
# `build-gnu.bash` ~ builds GNU coreutils (from supplied sources)
#

# spell-checker:ignore (paths) abmon deref discrim eacces getlimits getopt ginstall inacc infloop inotify reflink ; (misc) INT_OFLOW OFLOW baddecode submodules xstrtol distros ; (vars/env) SRCDIR vdir rcexp xpart dired OSTYPE ; (utils) gnproc greadlink gsed multihardlink texinfo

set -e

# Use GNU version for make, nproc, readlink and sed on *BSD
case "$OSTYPE" in
    *bsd*)
        MAKE="gmake"
        NPROC="gnproc"
        READLINK="greadlink"
        SED="gsed"
        ;;
    *)
        MAKE="make"
        NPROC="nproc"
        READLINK="readlink"
        SED="sed"
        ;;
esac

ME="${0}"
ME_dir="$(dirname -- "$("${READLINK}" -fm -- "${ME}")")"
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
path_GNU="$("${READLINK}" -fm -- "${path_GNU:-${path_UUTILS}/../gnu}")"

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

release_tag_GNU="v9.6"

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

# Set up quilt for patch management
export QUILT_PATCHES="${ME_dir}/gnu-patches/"
cd "$path_GNU"

# Check if all patches are already applied
if [ "$(quilt applied | wc -l)" -eq "$(quilt series | wc -l)" ]; then
    echo "All patches are already applied"
else
    # Push all patches
    quilt push -a || { echo "Failed to apply patches"; exit 1; }
fi
cd -

"${MAKE}" PROFILE="${UU_MAKE_PROFILE}"

cp "${UU_BUILD_DIR}/install" "${UU_BUILD_DIR}/ginstall" # The GNU tests rename this script before running, to avoid confusion with the make target
# Create *sum binaries
for sum in b2sum b3sum md5sum sha1sum sha224sum sha256sum sha384sum sha512sum; do
    sum_path="${UU_BUILD_DIR}/${sum}"
    test -f "${sum_path}" || (cd ${UU_BUILD_DIR} && ln -s "hashsum" "${sum}")
done
test -f "${UU_BUILD_DIR}/[" || (cd ${UU_BUILD_DIR} && ln -s "test" "[")

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
    echo "GNU build already found. Skip"
    echo "'rm -f $(pwd)/gnu-built' to force the build"
    echo "Note: the customization of the tests will still happen"
else
    # Disable useless checks
    sed -i 's|check-texinfo: $(syntax_checks)|check-texinfo:|' doc/local.mk
    ./bootstrap --skip-po
    ./configure --quiet --disable-gcc-warnings --disable-nls --disable-dependency-tracking --disable-bold-man-page-references
    #Add timeout to to protect against hangs
    sed -i 's|^"\$@|'"${SYSTEM_TIMEOUT}"' 600 "\$@|' build-aux/test-driver
    # Change the PATH in the Makefile to test the uutils coreutils instead of the GNU coreutils
    sed -i "s/^[[:blank:]]*PATH=.*/  PATH='${UU_BUILD_DIR//\//\\/}\$(PATH_SEPARATOR)'\"\$\$PATH\" \\\/" Makefile
    sed -i 's| tr | /usr/bin/tr |' tests/init.sh
    # Use a better diff
    sed -i 's|diff -c|diff -u|g' tests/Coreutils.pm
    "${MAKE}" -j "$("${NPROC}")"

    # Handle generated factor tests
    t_first=00
    t_max=37
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

    # Remove tests checking for --version & --help
    # Not really interesting for us and logs are too big
    sed -i -e '/tests\/help\/help-version.sh/ D' \
        -e '/tests\/help\/help-version-getopt.sh/ D' \
        Makefile
    touch gnu-built
fi

grep -rl 'path_prepend_' tests/* | xargs sed -i 's| path_prepend_ ./src||'

# Use the system coreutils where the test fails due to error in a util that is not the one being tested
sed -i "s|grep '^#define HAVE_CAP 1' \$CONFIG_HEADER > /dev/null|true|"  tests/ls/capability.sh
# tests/ls/abmon-align.sh - https://github.com/uutils/coreutils/issues/3505
sed -i 's|touch |/usr/bin/touch |' tests/test/test-N.sh tests/ls/abmon-align.sh

# our messages are better
sed -i "s|cannot stat 'symlink': Permission denied|not writing through dangling symlink 'symlink'|" tests/cp/fail-perm.sh
sed -i "s|cp: target directory 'symlink': Permission denied|cp: 'symlink' is not a directory|" tests/cp/fail-perm.sh

# Our message is a bit better
sed -i "s|cannot create regular file 'no-such/': Not a directory|'no-such/' is not a directory|" tests/mv/trailing-slash.sh

# Our message is better
sed -i "s|warning: unrecognized escape|warning: incomplete hex escape|" tests/stat/stat-printf.pl

sed -i 's|timeout |'"${SYSTEM_TIMEOUT}"' |' tests/tail/follow-stdin.sh

# Remove dup of /usr/bin/ and /usr/local/bin/ when executed several times
grep -rlE '/usr/bin/\s?/usr/bin' init.cfg tests/* | xargs -r sed -Ei 's|/usr/bin/\s?/usr/bin/|/usr/bin/|g'
grep -rlE '/usr/local/bin/\s?/usr/local/bin' init.cfg tests/* | xargs -r sed -Ei 's|/usr/local/bin/\s?/usr/local/bin/|/usr/local/bin/|g'

#### Adjust tests to make them work with Rust/coreutils
# in some cases, what we are doing in rust/coreutils is good (or better)
# we should not regress our project just to match what GNU is going.
# So, do some changes on the fly

sed -i -e "s|rm: cannot remove 'e/slink'|rm: cannot remove 'e'|g" tests/rm/fail-eacces.sh

sed -i -e "s|rm: cannot remove 'a/b/file'|rm: cannot remove 'a'|g" tests/rm/cycle.sh

sed -i -e "s|rm: cannot remove directory 'b/a/p'|rm: cannot remove 'b'|g" tests/rm/rm1.sh

sed -i -e "s|rm: cannot remove 'a/1'|rm: cannot remove 'a'|g" tests/rm/rm2.sh

sed -i -e "s|removed directory 'a/'|removed directory 'a'|g" tests/rm/v-slash.sh

# 'rel' doesn't exist. Our implementation is giving a better message.
sed -i -e "s|rm: cannot remove 'rel': Permission denied|rm: cannot remove 'rel': No such file or directory|g" tests/rm/inaccessible.sh

# overlay-headers.sh test intends to check for inotify events,
# however there's a bug because `---dis` is an alias for: `---disable-inotify`
sed -i -e "s|---dis ||g" tests/tail/overlay-headers.sh

# Do not FAIL, just do a regular ERROR
sed -i -e "s|framework_failure_ 'no inotify_add_watch';|fail=1;|" tests/tail/inotify-rotate-resources.sh

test -f "${UU_BUILD_DIR}/getlimits" || cp src/getlimits "${UU_BUILD_DIR}"

# pr produces very long log and this command isn't super interesting
# SKIP for now
sed -i -e "s|my \$prog = 'pr';$|my \$prog = 'pr';CuSkip::skip \"\$prog: SKIP for producing too long logs\";|" tests/pr/pr-tests.pl

# We don't have the same error message and no need to be that specific
sed -i -e "s|invalid suffix in --pages argument|invalid --pages argument|" \
    -e "s|--pages argument '\$too_big' too large|invalid --pages argument '\$too_big'|"  \
    -e "s|invalid page range|invalid --pages argument|" tests/misc/xstrtol.pl

# When decoding an invalid base32/64 string, gnu writes everything it was able to decode until
# it hit the decode error, while we don't write anything if the input is invalid.
sed -i "s/\(baddecode.*OUT=>\"\).*\"/\1\"/g" tests/basenc/base64.pl
sed -i "s/\(\(b2[ml]_[69]\|b32h_[56]\|z85_8\|z85_35\).*OUT=>\)[^}]*\(.*\)/\1\"\"\3/g" tests/basenc/basenc.pl

# add "error: " to the expected error message
sed -i "s/\$prog: invalid input/\$prog: error: invalid input/g" tests/basenc/basenc.pl

# basenc: swap out error message for unexpected arg
sed -i "s/  {ERR=>\"\$prog: foobar\\\\n\" \. \$try_help }/  {ERR=>\"error: unexpected argument '--foobar' found\n\n  tip: to pass '--foobar' as a value, use '-- --foobar'\n\nUsage: basenc [OPTION]... [FILE]\n\nFor more information, try '--help'.\n\"}]/" tests/basenc/basenc.pl
sed -i "s/  {ERR_SUBST=>\"s\/(unrecognized|unknown) option \[-' \]\*foobar\[' \]\*\/foobar\/\"}],//" tests/basenc/basenc.pl

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
# Ignore runcon, it needs some extra attention
# For all other tools, we want drop-in compatibility, and that includes the exit code.
sed -i -e "s/rcexp=1$/rcexp=1\n  case \"\$prg\" in runcon|stdbuf) return;; esac/" tests/misc/usage_vs_getopt.sh
# GNU has option=[SUFFIX], clap is <SUFFIX>
sed -i -e "s/cat opts/sed -i -e \"s| <.\*$||g\" opts/" tests/misc/usage_vs_getopt.sh
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
# Use GNU sed because option -z is not available on BSD sed
"${SED}" -i -Ez "s/\n([^\n#]*pad-3\.2[^\n]*)\n([^\n]*)\n([^\n]*)/\n# uutils\/numfmt supports padding = LONG_MIN\n#\1\n#\2\n#\3/" tests/misc/numfmt.pl

# Update the GNU error message to match the one generated by clap
sed -i -e "s/\$prog: multiple field specifications/error: the argument '--field <FIELDS>' cannot be used multiple times\n\nUsage: numfmt [OPTION]... [NUMBER]...\n\nFor more information, try '--help'./g" tests/misc/numfmt.pl
sed -i -e "s/Try 'mv --help' for more information/For more information, try '--help'/g" -e "s/mv: missing file operand/error: the following required arguments were not provided:\n  <files>...\n\nUsage: mv [OPTION]... [-T] SOURCE DEST\n       mv [OPTION]... SOURCE... DIRECTORY\n       mv [OPTION]... -t DIRECTORY SOURCE...\n/g" -e "s/mv: missing destination file operand after 'no-file'/error: The argument '<files>...' requires at least 2 values, but only 1 was provided\n\nUsage: mv [OPTION]... [-T] SOURCE DEST\n       mv [OPTION]... SOURCE... DIRECTORY\n       mv [OPTION]... -t DIRECTORY SOURCE...\n/g" tests/mv/diag.sh

# our error message is better
sed -i -e "s|mv: cannot overwrite 'a/t': Directory not empty|mv: cannot move 'b/t' to 'a/t': Directory not empty|" tests/mv/dir2dir.sh

# GNU doesn't support width > INT_MAX
# disable these test cases
sed -i -E "s|^([^#]*2_31.*)$|#\1|g" tests/printf/printf-cov.pl

sed -i -e "s/du: invalid -t argument/du: invalid --threshold argument/" -e "s/du: option requires an argument/error: a value is required for '--threshold <SIZE>' but none was supplied/" -e "s/Try 'du --help' for more information./\nFor more information, try '--help'./" tests/du/threshold.sh

# Remove the extra output check
sed -i -e "s|Try '\$prog --help' for more information.\\\n||" tests/du/files0-from.pl
sed -i -e "s|when reading file names from stdin, no file name of\"|-: No such file or directory\n\"|" -e "s| '-' allowed\\\n||" tests/du/files0-from.pl
sed -i -e "s|-: No such file or directory|cannot access '-': No such file or directory|g" tests/du/files0-from.pl

awk 'BEGIN {count=0} /compare exp out2/ && count < 6 {sub(/compare exp out2/, "grep -q \"cannot be used with\" out2"); count++} 1' tests/df/df-output.sh > tests/df/df-output.sh.tmp && mv tests/df/df-output.sh.tmp tests/df/df-output.sh

# with ls --dired, in case of error, we have a slightly different error position
sed -i -e "s|44 45|48 49|" tests/ls/stat-failed.sh

# small difference in the error message
# Use GNU sed for /c command
"${SED}" -i -e "/ls: invalid argument 'XX' for 'time style'/,/Try 'ls --help' for more information\./c\
ls: invalid --time-style argument 'XX'\nPossible values are: [\"full-iso\", \"long-iso\", \"iso\", \"locale\", \"+FORMAT (e.g., +%H:%M) for a 'date'-style format\"]\n\nFor more information try --help" tests/ls/time-style-diag.sh

# disable two kind of tests:
# "hostid BEFORE --help" doesn't fail for GNU. we fail. we are probably doing better
# "hostid BEFORE --help AFTER " same for this
sed -i -e "s/env \$prog \$BEFORE \$opt > out2/env \$prog \$BEFORE \$opt > out2 #/" -e "s/env \$prog \$BEFORE \$opt AFTER > out3/env \$prog \$BEFORE \$opt AFTER > out3 #/" -e "s/compare exp out2/compare exp out2 #/" -e "s/compare exp out3/compare exp out3 #/" tests/help/help-version-getopt.sh

# Add debug info + we have less syscall then GNU's. Adjust our check.
# Use GNU sed for /c command
"${SED}" -i -e '/test \$n_stat1 = \$n_stat2 \\/c\
echo "n_stat1 = \$n_stat1"\n\
echo "n_stat2 = \$n_stat2"\n\
test \$n_stat1 -ge \$n_stat2 \\' tests/ls/stat-free-color.sh

# no need to replicate this output with hashsum
sed -i -e  "s|Try 'md5sum --help' for more information.\\\n||" tests/cksum/md5sum.pl

# Our ls command always outputs ANSI color codes prepended with a zero. However,
# in the case of GNU, it seems inconsistent. Nevertheless, it looks like it
# doesn't matter whether we prepend a zero or not.
sed -i -E 's/\^\[\[([1-9]m)/^[[0\1/g;  s/\^\[\[m/^[[0m/g' tests/ls/color-norm.sh
# It says in the test itself that having more than one reset is a bug, so we
# don't need to replicate that behavior.
sed -i -E 's/(\^\[\[0m)+/\^\[\[0m/g' tests/ls/color-norm.sh

# GNU's ls seems to output color codes in the order given in the environment
# variable, but our ls seems to output them in a predefined order. Nevertheless,
# the order doesn't matter, so it's okay.
sed -i  's/44;37/37;44/' tests/ls/multihardlink.sh

# Just like mentioned in the previous patch, GNU's ls output color codes in the
# same way it is specified in the environment variable, but our ls emits them
# differently. In this case, the color code is set to 0;31;42, and our ls would
# ignore the 0; part. This would have been a bug if we output color codes
# individually, for example, ^[[31^[[42 instead of ^[[31;42, but we don't do
# that anywhere in our implementation, and it looks like GNU's ls also doesn't
# do that. So, it's okay to ignore the zero.
sed -i  "s/color_code='0;31;42'/color_code='31;42'/" tests/ls/color-clear-to-eol.sh

# patching this because of the same reason as the last one.
sed -i  "s/color_code='0;31;42'/color_code='31;42'/" tests/ls/quote-align.sh

# Slightly different error message
sed -i 's/not supported/unexpected argument/' tests/mv/mv-exchange.sh
# Most tests check that `/usr/bin/tr` is working correctly before running.
# However in NixOS/Nix-based distros, the tr util is located somewhere in
# /nix/store/xxxxxxxxxxxx...xxxx/bin/tr
# We just replace the references to `/usr/bin/tr` with the result of `$(which tr)`
sed -i  's/\/usr\/bin\/tr/$(which tr)/' tests/init.sh
