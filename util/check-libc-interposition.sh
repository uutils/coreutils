#!/bin/bash
#
# Check that utilities reach the kernel through libc symbols rather than raw
# syscalls.
#
# LD_PRELOAD wrappers (fakeroot, fakechroot, pseudo) are an essential part of
# distribution build tooling, and they interpose libc symbols. A hand-written
# syscall(2) goes around them: the change lands on disk, but their bookkeeping
# never sees it, so every later stat() reports the old state -- silently, with
# an empty stderr and exit status 0 (issue #14028).
#
# spell-checker:ignore fakeroot fakechroot

set -e

echo "=== libc Interposition Verification ==="

# shellcheck disable=SC2034  # read by check-common.sh once sourced
CHECK_UTILS="chmod chown"
. "$(dirname "${BASH_SOURCE[0]}")/check-common.sh"

require_command fakeroot

# Run a utility under fakeroot and compare what fakeroot's database reports
# afterwards against what the utility asked for. A raw syscall leaves the
# database stale, so the observed value is the one from before the run.
#
# $1 test name, $2 command to run (relative to the tree), $3 stat format,
# $4 expected value
assert_visible_under_fakeroot() {
    local name="$1" cmd="$2" format="$3" expected="$4" observed tree

    tree="$TEMP_DIR/$name"
    mkdir -p "$tree/vendor"

    # The chown makes fakeroot track both inodes, so what it reports back comes
    # from its database rather than straight from the filesystem.
    observed=$(cd "$tree" && fakeroot sh -c \
        "$CHOWN_CMD -R 7:11 . && $cmd && /usr/bin/stat -c $format vendor")

    if [ "$observed" != "$expected" ]; then
        fail_immediately "$name: fakeroot reports '$observed', expected '$expected' -- the change must go through a libc symbol, not a raw syscall (issue #14028)"
    fi
    echo "✓ $name is visible to LD_PRELOAD wrappers"
}

# chown itself is the vehicle for every other check, so it has to be present.
CHOWN_CMD=$(util_cmd chown) || {
    echo "Error: chown was not built, cannot set up the checks."
    echo "Build it with 'cargo build --profile=${PROFILE} -p uu_chmod -p uu_chown'"
    exit 1
}
assert_visible_under_fakeroot "chown -R" "true" "%u:%g" "7:11"

if CHMOD_CMD=$(util_cmd chmod); then
    assert_visible_under_fakeroot "chmod -R" "$CHMOD_CMD -R 2751 ." "%a" "2751"
else
    echo "⚠ chmod not built, skipping"
fi

echo ""
echo "=== Summary ==="
echo "All checked utilities go through libc!"
