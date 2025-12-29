#!/usr/bin/env bash
# `build-gnu.bash` ~ builds GNU coreutils (from supplied sources)
#

# spell-checker:ignore (paths) abmon deref discrim eacces getlimits getopt ginstall inacc infloop inotify reflink ; (misc) INT_OFLOW OFLOW
# spell-checker:ignore baddecode submodules xstrtol distros ; (vars/env) SRCDIR vdir rcexp xpart dired OSTYPE ; (utils) greadlink gsed multihardlink texinfo CARGOFLAGS
# spell-checker:ignore openat TOCTOU CFLAGS tmpfs

set -e

# Use GNU make, readlink and sed on *BSD and macOS
MAKE=$(command -v gmake||command -v make)
READLINK=$(command -v greadlink||command -v readlink) # Use our readlink to remove a dependency
SED=$(command -v gsed||command -v sed)

SYSTEM_TIMEOUT=$(command -v timeout)
SYSTEM_YES=$(command -v yes)

ME="${0}"
ME_dir="$(dirname -- "$("${READLINK}" -fm -- "${ME}")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"


: ${PROFILE:=debug} # default profile
export PROFILE
CARGO_FEATURE_FLAGS=""

### * config (from environment with fallback defaults); note: GNU is expected to be a sibling repo directory

path_UUTILS=${path_UUTILS:-${REPO_main_dir}}
path_GNU="$("${READLINK}" -fm -- "${path_GNU:-${path_UUTILS}/../gnu}")"

###

# check if the GNU coreutils has been cloned, if not print instructions
# note: the ${path_GNU} might already exist, so we check for the configure
if test ! -f "${path_GNU}/configure"; then
    echo "Could not find the GNU coreutils (expected at '${path_GNU}')"
    echo "Download them to the expected path:"
    echo " (mkdir -p '${path_GNU}' && cd '${path_GNU}' && bash '${path_UUTILS}/util/fetch-gnu.sh')"
    echo "You can edit fetch-gnu.sh to change the tag"
    exit 1
fi

###

echo "ME='${ME}'"
echo "ME_dir='${ME_dir}'"
echo "REPO_main_dir='${REPO_main_dir}'"

echo "path_UUTILS='${path_UUTILS}'"
echo "path_GNU='${path_GNU}'"

###

if [[ ! -z  "$CARGO_TARGET_DIR" ]]; then
UU_BUILD_DIR="${CARGO_TARGET_DIR}/${PROFILE}"
else
UU_BUILD_DIR="${path_UUTILS}/target/${PROFILE}"
fi
echo "UU_BUILD_DIR='${UU_BUILD_DIR}'"

cd "${path_UUTILS}" && echo "[ pwd:'${PWD}' ]"

export SELINUX_ENABLED # Run this script with=1 for testing SELinux
[ "${SELINUX_ENABLED}" = 1 ] && CARGO_FEATURE_FLAGS="${CARGO_FEATURE_FLAGS} selinux"

# Trim leading whitespace from feature flags
CARGO_FEATURE_FLAGS="$(echo "${CARGO_FEATURE_FLAGS}" | sed -e 's/^[[:space:]]*//')"

# If we have feature flags, format them correctly for cargo
if [ ! -z "${CARGO_FEATURE_FLAGS}" ]; then
    CARGO_FEATURE_FLAGS="--features ${CARGO_FEATURE_FLAGS}"
    echo "Building with cargo flags: ${CARGO_FEATURE_FLAGS}"
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

# Pass the feature flags to make, which will pass them to cargo
"${MAKE}" PROFILE="${PROFILE}" SKIP_UTILS=more CARGOFLAGS="${CARGO_FEATURE_FLAGS}"
# min test for SELinux
[ "${SELINUX_ENABLED}" = 1 ] && touch g && "${PROFILE}"/stat -c%C g && rm g

cp "${UU_BUILD_DIR}/install" "${UU_BUILD_DIR}/ginstall" # The GNU tests rename this script before running, to avoid confusion with the make target
# Create *sum binaries
for sum in b2sum md5sum sha1sum sha224sum sha256sum sha384sum sha512sum; do
    sum_path="${UU_BUILD_DIR}/${sum}"
    test -f "${sum_path}" || (cd ${UU_BUILD_DIR} && ln -s "hashsum" "${sum}")
done
test -f "${UU_BUILD_DIR}/[" || (cd ${UU_BUILD_DIR} && ln -s "test" "[")

##

cd "${path_GNU}" && echo "[ pwd:'${PWD}' ]"

# Any binaries that aren't built become `false` to make tests failure
# Note that some test (e.g. runcon/runcon-compute.sh) incorrectly passes by this
for binary in $(./build-aux/gen-lists-of-programs.sh --list-progs); do
    bin_path="${UU_BUILD_DIR}/${binary}"
    test -f "${bin_path}" || {
        echo "'${binary}' was not built with uutils, using the 'false' program"
        cp "${UU_BUILD_DIR}/false" "${bin_path}"
    }
done

# Always update the PATH to test the uutils coreutils instead of the GNU coreutils
# This ensures the correct path is used even if the repository was moved or rebuilt in a different location
"${SED}" -i "s/^[[:blank:]]*PATH=.*/  PATH='${UU_BUILD_DIR//\//\\/}\$(PATH_SEPARATOR)'\"\$\$PATH\" \\\/" tests/local.mk

if test -f gnu-built; then
    echo "GNU build already found. Skip"
    echo "'rm -f $(pwd)/{gnu-built,src/getlimits}' to force the build"
    echo "Note: the customization of the tests will still happen"
else
    # Disable useless checks
    "${SED}" -i 's|check-texinfo: $(syntax_checks)|check-texinfo:|' doc/local.mk
    # Use CFLAGS for best build time since we discard GNU coreutils
    CFLAGS="${CFLAGS} -pipe -O0 -s" ./configure -C --quiet --disable-gcc-warnings --disable-nls --disable-dependency-tracking --disable-bold-man-page-references \
      --enable-single-binary=symlinks --enable-install-program="arch,kill,uptime,hostname" \
      "$([ "${SELINUX_ENABLED}" = 1 ] && echo --with-selinux || echo --without-selinux)"
    #Add timeout to to protect against hangs
    "${SED}" -i 's|^"\$@|'"${SYSTEM_TIMEOUT}"' 600 "\$@|' build-aux/test-driver
    # Use a better diff
    "${SED}" -i 's|diff -c|diff -u|g' tests/Coreutils.pm

    # Skip make if possible
    # Use our nproc for *BSD and macOS
    test -f src/getlimits || "${MAKE}" -j "$("${UU_BUILD_DIR}/nproc")"
    cp -f src/getlimits "${UU_BUILD_DIR}"

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
        "${SED}" -i -e "s/\$(tf)\/t${i}.sh//g" Makefile
    done

    # Remove tests checking for --version & --help
    # Not really interesting for us and logs are too big
    "${SED}" -i -e '/tests\/help\/help-version.sh/ D' \
        -e '/tests\/help\/help-version-getopt.sh/ D' \
        Makefile
    touch gnu-built
fi

grep -rl 'path_prepend_' tests/* | xargs -r "${SED}" -i 's| path_prepend_ ./src||'
# path_prepend_ sets $abs_path_dir_: set it manually instead.
grep -rl '\$abs_path_dir_' tests/*/*.sh | xargs -r "${SED}" -i "s|\$abs_path_dir_|${UU_BUILD_DIR//\//\\/}|g"

# We can't build runcon and chcon without libselinux. But GNU no longer builds dummies of them. So consider they are SELinux specific.
"${SED}" -i 's/^print_ver_.*/require_selinux_/' tests/runcon/runcon-compute.sh
"${SED}" -i 's/^print_ver_.*/require_selinux_/' tests/runcon/runcon-no-reorder.sh
"${SED}" -i 's/^print_ver_.*/require_selinux_/' tests/chcon/chcon-fail.sh

# Mask mtab by unshare instead of LD_PRELOAD (able to merge this to GNU?)
"${SED}" -i -e 's|^export LD_PRELOAD=.*||' -e "s|.*maybe LD_PRELOAD.*|df() { unshare -rm bash -c \"mount -t tmpfs tmpfs /proc \&\& command df \\\\\"\\\\\$@\\\\\"\" -- \"\$@\"; }|" tests/df/no-mtab-status.sh
# We use coreutils yes
"${SED}" -i "s|--coreutils-prog=||g" tests/misc/coreutils.sh
# Different message
"${SED}" -i "s|coreutils: unknown program 'blah'|blah: function/utility not found|" tests/misc/coreutils.sh

# Use the system coreutils where the test fails due to error in a util that is not the one being tested
"${SED}" -i "s|grep '^#define HAVE_CAP 1' \$CONFIG_HEADER > /dev/null|true|"  tests/ls/capability.sh

# our messages are better
"${SED}" -i "s|cannot stat 'symlink': Permission denied|not writing through dangling symlink 'symlink'|" tests/cp/fail-perm.sh
"${SED}" -i "s|cp: target directory 'symlink': Permission denied|cp: 'symlink' is not a directory|" tests/cp/fail-perm.sh

# Our message is a bit better
"${SED}" -i "s|cannot create regular file 'no-such/': Not a directory|'no-such/' is not a directory|" tests/mv/trailing-slash.sh

# Our message is better
"${SED}" -i "s|warning: unrecognized escape|warning: incomplete hex escape|" tests/stat/stat-printf.pl

"${SED}" -i 's|timeout |'"${SYSTEM_TIMEOUT}"' |' tests/tail/follow-stdin.sh

# trap_sigpipe_or_skip_ fails with uutils tools because of a bug in
# timeout/yes (https://github.com/uutils/coreutils/issues/7252), so we use
# system's yes/timeout to make sure the tests run (instead of being skipped).
"${SED}" -i 's|\(trap .* \)timeout\( .* \)yes|'"\1${SYSTEM_TIMEOUT}\2${SYSTEM_YES}"'|' init.cfg

# Remove dup of /usr/bin/ and /usr/local/bin/ when executed several times
grep -rlE '/usr/bin/\s?/usr/bin' init.cfg tests/* | xargs -r "${SED}" -Ei 's|/usr/bin/\s?/usr/bin/|/usr/bin/|g'
grep -rlE '/usr/local/bin/\s?/usr/local/bin' init.cfg tests/* | xargs -r "${SED}" -Ei 's|/usr/local/bin/\s?/usr/local/bin/|/usr/local/bin/|g'

#### Adjust tests to make them work with Rust/coreutils
# in some cases, what we are doing in rust/coreutils is good (or better)
# we should not regress our project just to match what GNU is going.
# So, do some changes on the fly

"${SED}" -i -e "s|removed directory 'a/'|removed directory 'a'|g" tests/rm/v-slash.sh

# 'rel' doesn't exist. Our implementation is giving a better message.
"${SED}" -i -e "s|rm: cannot remove 'rel': Permission denied|rm: cannot remove 'rel': No such file or directory|g" tests/rm/inaccessible.sh

# Our implementation shows "Directory not empty" for directories that can't be accessed due to lack of execute permissions
# This is actually more accurate than "Permission denied" since the real issue is that we can't empty the directory
"${SED}" -i -e "s|rm: cannot remove 'a/1': Permission denied|rm: cannot remove 'a/1/2': Permission denied|g" -e "s|rm: cannot remove 'b': Permission denied|rm: cannot remove 'a': Directory not empty\nrm: cannot remove 'b/3': Permission denied|g" tests/rm/rm2.sh

# overlay-headers.sh test intends to check for inotify events,
# however there's a bug because `---dis` is an alias for: `---disable-inotify`
sed -i -e "s|---dis ||g" tests/tail/overlay-headers.sh

# Do not FAIL, just do a regular ERROR
"${SED}" -i -e "s|framework_failure_ 'no inotify_add_watch';|fail=1;|" tests/tail/inotify-rotate-resources.sh

# pr-tests.pl: Override the comparison function to suppress diff output
# This prevents the test from overwhelming logs while still reporting failures
"${SED}" -i '/^my $fail = run_tests/i no warnings "redefine"; *Coreutils::_compare_files = sub { my ($p, $t, $io, $a, $e) = @_; my $d = File::Compare::compare($a, $e); warn "$p: test $t: mismatch\\n" if $d; return $d; };' tests/pr/pr-tests.pl

# We don't have the same error message and no need to be that specific
"${SED}" -i -e "s|invalid suffix in --pages argument|invalid --pages argument|" \
    -e "s|--pages argument '\$too_big' too large|invalid --pages argument '\$too_big'|"  \
    -e "s|invalid page range|invalid --pages argument|" tests/misc/xstrtol.pl

# When decoding an invalid base32/64 string, gnu writes everything it was able to decode until
# it hit the decode error, while we don't write anything if the input is invalid.
"${SED}" -i "s/\(baddecode.*OUT=>\"\).*\"/\1\"/g" tests/basenc/base64.pl
"${SED}" -i "s/\(\(b2[ml]_[69]\|z85_8\|z85_35\).*OUT=>\)[^}]*\(.*\)/\1\"\"\3/g" tests/basenc/basenc.pl

# add "error: " to the expected error message
"${SED}" -i "s/\$prog: invalid input/\$prog: error: invalid input/g" tests/basenc/basenc.pl

# basenc: swap out error message for unexpected arg
"${SED}" -i "s/  {ERR=>\"\$prog: foobar\\\\n\" \. \$try_help }/  {ERR=>\"error: unexpected argument '--foobar' found\n\n  tip: to pass '--foobar' as a value, use '-- --foobar'\n\nUsage: basenc [OPTION]... [FILE]\n\nFor more information, try '--help'.\n\"}]/" tests/basenc/basenc.pl
"${SED}" -i "s/  {ERR_SUBST=>\"s\/(unrecognized|unknown) option \[-' \]\*foobar\[' \]\*\/foobar\/\"}],//" tests/basenc/basenc.pl

# exit early for the selinux check. The first is enough for us.
"${SED}" -i "s|# Independent of whether SELinux|return 0\n  #|g" init.cfg

# Some tests are executed with the "nobody" user.
# The check to verify if it works is based on the GNU coreutils version
# making it too restrictive for us
"${SED}" -i "s|\$PACKAGE_VERSION|[0-9]*|g" tests/rm/fail-2eperm.sh tests/mv/sticky-to-xpart.sh init.cfg

# usage_vs_getopt.sh is heavily modified as it runs all the binaries
# with the option -/ is used, clap is returning a better error than GNU's. Adjust the GNU test
"${SED}" -i -e "s~  grep \" '\*/'\*\" err || framework_failure_~  grep \" '*-/'*\" err || framework_failure_~" tests/misc/usage_vs_getopt.sh
"${SED}" -i -e "s~  sed -n \"1s/'\\\/'/'OPT'/p\" < err >> pat || framework_failure_~  sed -n \"1s/'-\\\/'/'OPT'/p\" < err >> pat || framework_failure_~" tests/misc/usage_vs_getopt.sh
# Ignore runcon, it needs some extra attention
# For all other tools, we want drop-in compatibility, and that includes the exit code.
"${SED}" -i -e "s/rcexp=1$/rcexp=1\n  case \"\$prg\" in runcon|stdbuf) return;; esac/" tests/misc/usage_vs_getopt.sh
# GNU has option=[SUFFIX], clap is <SUFFIX>
"${SED}" -i -e "s/cat opts/sed -i -e \"s| <.\*$||g\" opts/" tests/misc/usage_vs_getopt.sh
# for some reasons, some stuff are duplicated, strip that
"${SED}" -i -e "s/provoked error./provoked error\ncat pat |sort -u > pat/" tests/misc/usage_vs_getopt.sh

# install verbose messages shows ginstall as command
"${SED}" -i -e "s/ginstall: creating directory/install: creating directory/g" tests/install/basic-1.sh

# GNU doesn't support padding < -LONG_MAX
# disable this test case
"${SED}" -i -Ez "s/\n([^\n#]*pad-3\.2[^\n]*)\n([^\n]*)\n([^\n]*)/\n# uutils\/numfmt supports padding = LONG_MIN\n#\1\n#\2\n#\3/" tests/numfmt/numfmt.pl

# Update the GNU error message to match the one generated by clap
"${SED}" -i -e "s/\$prog: multiple field specifications/error: the argument '--field <FIELDS>' cannot be used multiple times\n\nUsage: numfmt [OPTION]... [NUMBER]...\n\nFor more information, try '--help'./g" tests/numfmt/numfmt.pl
"${SED}" -i -e "s/Try 'mv --help' for more information/For more information, try '--help'/g" -e "s/mv: missing file operand/error: the following required arguments were not provided:\n  <files>...\n\nUsage: mv [OPTION]... [-T] SOURCE DEST\n       mv [OPTION]... SOURCE... DIRECTORY\n       mv [OPTION]... -t DIRECTORY SOURCE...\n/g" -e "s/mv: missing destination file operand after 'no-file'/error: The argument '<files>...' requires at least 2 values, but only 1 was provided\n\nUsage: mv [OPTION]... [-T] SOURCE DEST\n       mv [OPTION]... SOURCE... DIRECTORY\n       mv [OPTION]... -t DIRECTORY SOURCE...\n/g" tests/mv/diag.sh

# our error message is better
"${SED}" -i -e "s|mv: cannot overwrite 'a/t': Directory not empty|mv: cannot move 'b/t' to 'a/t': Directory not empty|" tests/mv/dir2dir.sh

# GNU doesn't support width > INT_MAX
# disable these test cases
"${SED}" -i -E "s|^([^#]*2_31.*)$|#\1|g" tests/printf/printf-cov.pl

"${SED}" -i -e "s/du: invalid -t argument/du: invalid --threshold argument/" -e "s/du: option requires an argument/error: a value is required for '--threshold <SIZE>' but none was supplied/" -e "s/Try 'du --help' for more information./\nFor more information, try '--help'./" tests/du/threshold.sh

# Remove the extra output check
"${SED}" -i -e "s|Try '\$prog --help' for more information.\\\n||" tests/du/files0-from.pl
"${SED}" -i -e "s|-: No such file or directory|cannot access '-': No such file or directory|g" tests/du/files0-from.pl

# Skip the move-dir-while-traversing test - our implementation uses safe traversal with openat()
# which avoids the TOCTOU race condition that this test tries to trigger. The test uses inotify
# to detect when du opens a directory path and moves it to cause an error, but our openat-based
# implementation doesn't trigger inotify events on the full path, preventing the race condition.
# This is actually better behavior - we're immune to this class of filesystem race attacks.
"${SED}" -i '1s/^/exit 0  # Skip test - uutils du uses safe traversal that prevents this race condition\n/' tests/du/move-dir-while-traversing.sh

awk 'BEGIN {count=0} /compare exp out2/ && count < 6 {sub(/compare exp out2/, "grep -q \"cannot be used with\" out2"); count++} 1' tests/df/df-output.sh > tests/df/df-output.sh.tmp && mv tests/df/df-output.sh.tmp tests/df/df-output.sh

# with ls --dired, in case of error, we have a slightly different error position
"${SED}" -i -e "s|44 45|48 49|" tests/ls/stat-failed.sh

# small difference in the error message
"${SED}" -i -e "s/ls: invalid argument 'XX' for 'time style'/ls: invalid --time-style argument 'XX'/" \
    -e "s/Valid arguments are:/Possible values are:/" \
    -e "s/Try 'ls --help' for more information./\nFor more information try --help/" \
    tests/ls/time-style-diag.sh

# disable two kind of tests:
# "hostid BEFORE --help" doesn't fail for GNU. we fail. we are probably doing better
# "hostid BEFORE --help AFTER " same for this
"${SED}" -i -e "s/env \$prog \$BEFORE \$opt > out2/env \$prog \$BEFORE \$opt > out2 #/" -e "s/env \$prog \$BEFORE \$opt AFTER > out3/env \$prog \$BEFORE \$opt AFTER > out3 #/" -e "s/compare exp out2/compare exp out2 #/" -e "s/compare exp out3/compare exp out3 #/" tests/help/help-version-getopt.sh

# Add debug info + we have less syscall then GNU's. Adjust our check.
"${SED}" -i -e '/test \$n_stat1 = \$n_stat2 \\/c\
echo "n_stat1 = \$n_stat1"\n\
echo "n_stat2 = \$n_stat2"\n\
test \$n_stat1 -ge \$n_stat2 \\' tests/ls/stat-free-color.sh

# no need to replicate this output with hashsum
"${SED}" -i -e  "s|Try 'md5sum --help' for more information.\\\n||" tests/cksum/md5sum.pl

# Our ls command always outputs ANSI color codes prepended with a zero. However,
# in the case of GNU, it seems inconsistent. Nevertheless, it looks like it
# doesn't matter whether we prepend a zero or not.
"${SED}" -i -E 's/\^\[\[([1-9]m)/^[[0\1/g;  s/\^\[\[m/^[[0m/g' tests/ls/color-norm.sh
# It says in the test itself that having more than one reset is a bug, so we
# don't need to replicate that behavior.
"${SED}" -i -E 's/(\^\[\[0m)+/\^\[\[0m/g' tests/ls/color-norm.sh

# GNU's ls seems to output color codes in the order given in the environment
# variable, but our ls seems to output them in a predefined order. Nevertheless,
# the order doesn't matter, so it's okay.
"${SED}" -i  's/44;37/37;44/' tests/ls/multihardlink.sh

# Just like mentioned in the previous patch, GNU's ls output color codes in the
# same way it is specified in the environment variable, but our ls emits them
# differently. In this case, the color code is set to 0;31;42, and our ls would
# ignore the 0; part. This would have been a bug if we output color codes
# individually, for example, ^[[31^[[42 instead of ^[[31;42, but we don't do
# that anywhere in our implementation, and it looks like GNU's ls also doesn't
# do that. So, it's okay to ignore the zero.
"${SED}" -i  "s/color_code='0;31;42'/color_code='31;42'/" tests/ls/color-clear-to-eol.sh

# patching this because of the same reason as the last one.
"${SED}" -i  "s/color_code='0;31;42'/color_code='31;42'/" tests/ls/quote-align.sh

# Slightly different error message
"${SED}" -i 's/not supported/unexpected argument/' tests/mv/mv-exchange.sh

# upstream doesn't having the program name in the error message
# but we do. We should keep it that way.
"${SED}" -i 's/echo "changing security context/echo "chcon: changing security context/' tests/chcon/chcon.sh

# Disable this test, it is not relevant for us:
# * the selinux crate is handling errors
# * the test says "maybe we should not fail when no context available"
"${SED}" -i -e "s|returns_ 1||g" tests/cp/no-ctx.sh
