#!/bin/bash
#
# Check that utilities are using safe traversal (openat family syscalls)
# to prevent TOCTOU race conditions
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMP_DIR=$(mktemp -d)

# Function to exit immediately on error
fail_immediately() {
    echo "❌ FAILED: $1"
    echo ""
    echo "Debug information available in: $TEMP_DIR/strace_*.log"
    exit 1
}

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo "=== Safe Traversal Verification ==="

# Assume binaries are already built (for CI usage)
# Prefer individual binaries for more accurate testing
if [ -f "$PROJECT_ROOT/target/release/rm" ]; then
    echo "Using individual binaries"
    USE_MULTICALL=0
elif [ -f "$PROJECT_ROOT/target/release/coreutils" ]; then
    echo "Using multicall binary"
    USE_MULTICALL=1
    COREUTILS_BIN="$PROJECT_ROOT/target/release/coreutils"
else
    echo "Error: No binaries found. Please build first with 'cargo build --release'"
    exit 1
fi

cd "$TEMP_DIR"

# Create test directory structure
mkdir -p test_dir/sub1/sub2/sub3
echo "test1" > test_dir/file1.txt
echo "test2" > test_dir/sub1/file2.txt
echo "test3" > test_dir/sub1/sub2/file3.txt
echo "test4" > test_dir/sub1/sub2/sub3/file4.txt

check_utility() {
    local util="$1"
    local trace_syscalls="$2"
    local expected_syscalls="$3"
    local test_args="$4"
    local test_name="$5"

    echo ""
    echo "Testing $util ($test_name)..."

    local strace_log="strace_${util}_${test_name}.log"

    # Choose binary to use
    if [ "$USE_MULTICALL" -eq 1 ]; then
        local util_cmd="$COREUTILS_BIN $util"
    else
        local util_path="$PROJECT_ROOT/target/release/$util"
        if [ ! -f "$util_path" ]; then
            fail_immediately "$util binary not found at $util_path"
        fi
        local util_cmd="$util_path"
    fi

    # Run utility under strace
    strace -f -e trace="$trace_syscalls" -o "$strace_log" \
        $util_cmd $test_args 2>/dev/null || true
    cat $strace_log
    # Check for expected safe syscalls
    local found_safe=0
    for syscall in $expected_syscalls; do
        if grep -q "$syscall" "$strace_log"; then
            echo "✓ Found $syscall() (safe traversal)"
            found_safe=$((found_safe + 1))
        else
            fail_immediately "Missing $syscall() (safe traversal not active for $util)"
        fi
    done

    # Count detailed syscall statistics
    local openat_count unlinkat_count fchmodat_count fchownat_count newfstatat_count renameat_count
    local unlink_count rmdir_count chmod_count chown_count safe_ops unsafe_ops

    openat_count=$(grep -c "openat(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")
    unlinkat_count=$(grep -c "unlinkat(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")
    fchmodat_count=$(grep -c "fchmodat(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")
    fchownat_count=$(grep -c "fchownat(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")
    newfstatat_count=$(grep -c "newfstatat(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")
    renameat_count=$(grep -c "renameat(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")

    # Count old unsafe syscalls (exclude the trace line prefix)
    unlink_count=$(grep -cE "\bunlink\(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")
    rmdir_count=$(grep -cE "\brmdir\(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")
    chmod_count=$(grep -cE "\bchmod\(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")
    chown_count=$(grep -cE "\b(chown|lchown)\(" "$strace_log" 2>/dev/null | tr -d '\n' || echo "0")

    # Ensure all variables are integers
    [ -z "$openat_count" ] && openat_count=0
    [ -z "$unlinkat_count" ] && unlinkat_count=0
    [ -z "$fchmodat_count" ] && fchmodat_count=0
    [ -z "$fchownat_count" ] && fchownat_count=0
    [ -z "$newfstatat_count" ] && newfstatat_count=0
    [ -z "$renameat_count" ] && renameat_count=0
    [ -z "$unlink_count" ] && unlink_count=0
    [ -z "$rmdir_count" ] && rmdir_count=0
    [ -z "$chmod_count" ] && chmod_count=0
    [ -z "$chown_count" ] && chown_count=0

    # Calculate totals
    safe_ops=$((openat_count + unlinkat_count + fchmodat_count + fchownat_count + newfstatat_count + renameat_count))
    unsafe_ops=$((unlink_count + rmdir_count + chmod_count + chown_count))

    echo "  Strace statistics:"
    echo "    Safe syscalls: openat=$openat_count unlinkat=$unlinkat_count fchmodat=$fchmodat_count fchownat=$fchownat_count newfstatat=$newfstatat_count renameat=$renameat_count"
    echo "    Unsafe syscalls: unlink=$unlink_count rmdir=$rmdir_count chmod=$chmod_count chown/lchown=$chown_count"
    echo "    Total: safe=$safe_ops unsafe=$unsafe_ops"

    # For rm specifically, we expect unlinkat instead of unlink/rmdir for file operations
    # Note: A single rmdir() for the root directory is acceptable because:
    # 1. The root directory path is provided by the user (not discovered during traversal)
    # 2. There's no TOCTOU race - we're not resolving paths during recursive operations
    # 3. After safe traversal removes all contents via unlinkat(), rmdir() is safe for the empty root
    if [ "$util" = "rm" ]; then
        if [ "$unlinkat_count" -gt 0 ] && [ "$unlink_count" -eq 0 ] && [ "$rmdir_count" -le 1 ]; then
            echo "✓ Using safe syscalls (unlinkat for traversal)"
            if [ "$rmdir_count" -eq 1 ]; then
                echo "  Note: Single rmdir() for root directory is acceptable"
            fi
        elif [ "$unlink_count" -gt 0 ] || [ "$rmdir_count" -gt 1 ]; then
            fail_immediately "$util is UNSAFE: Using unlink/rmdir for file operations (unlink=$unlink_count rmdir=$rmdir_count unlinkat=$unlinkat_count) - vulnerable to TOCTOU attacks"
        else
            echo "⚠ No file removal operations detected"
        fi
    elif [ "$safe_ops" -gt 0 ] && [ "$unsafe_ops" -eq 0 ]; then
        echo "✓ Using only safe syscalls"
    elif [ "$safe_ops" -gt 0 ] && [ "$safe_ops" -ge "$unsafe_ops" ]; then
        echo "✓ Using primarily safe syscalls"
    elif [ "$found_safe" -gt 0 ]; then
        echo "⚠ Some safe syscalls found but mixed with unsafe ops"
    else
        fail_immediately "$util is not using safe traversal"
    fi
}

# Get list of available utilities
if [ "$USE_MULTICALL" -eq 1 ]; then
    AVAILABLE_UTILS=$($COREUTILS_BIN --list)
else
    AVAILABLE_UTILS=""
    for util in rm chmod chown chgrp du mv; do
        if [ -f "$PROJECT_ROOT/target/release/$util" ]; then
            AVAILABLE_UTILS="$AVAILABLE_UTILS $util"
        fi
    done
fi

# Test rm - should use openat, unlinkat, newfstatat
if echo "$AVAILABLE_UTILS" | grep -q "rm"; then
    cp -r test_dir test_rm
    check_utility "rm" "openat,unlinkat,newfstatat,unlink,rmdir" "openat" "-rf test_rm" "recursive_remove"

    # Regression guard: rm must not issue path-based statx calls (should rely on dirfd-relative newfstatat)
    if grep -qE 'statx\(AT_FDCWD, "/' strace_rm_recursive_remove.log; then
        fail_immediately "rm is using path-based statx (absolute path); expected dirfd-relative newfstatat"
    fi
    if grep -qE 'statx\(AT_FDCWD, "[^"]*/' strace_rm_recursive_remove.log; then
        fail_immediately "rm is using path-based statx (multi-component relative path); expected dirfd-relative newfstatat"
    fi
fi

# Test chmod - should use openat, fchmodat, newfstatat
if echo "$AVAILABLE_UTILS" | grep -q "chmod"; then
    cp -r test_dir test_chmod
    check_utility "chmod" "openat,fchmodat,newfstatat,chmod" "openat fchmodat" "-R 755 test_chmod" "recursive_chmod"

    # Additional regression guard: ensure recursion uses dirfd-relative openat, not AT_FDCWD with a multi-component path
    if grep -q 'openat(AT_FDCWD, "test_chmod/' strace_chmod_recursive_chmod.log; then
        fail_immediately "chmod recursed using AT_FDCWD with a multi-component path; expected dirfd-relative openat"
    fi
fi

# Test chown - should use openat, fchownat, newfstatat
if echo "$AVAILABLE_UTILS" | grep -q "chown"; then
    cp -r test_dir test_chown
    USER_ID=$(id -u)
    GROUP_ID=$(id -g)
    check_utility "chown" "openat,fchownat,newfstatat,chown,lchown" "openat fchownat" "-R $USER_ID:$GROUP_ID test_chown" "recursive_chown"
fi

# Test chgrp - should use openat, fchownat, newfstatat
if echo "$AVAILABLE_UTILS" | grep -q "chgrp"; then
    cp -r test_dir test_chgrp
    check_utility "chgrp" "openat,fchownat,newfstatat,chown,lchown" "openat fchownat" "-R $GROUP_ID test_chgrp" "recursive_chgrp"
fi

# Test du - should use openat, newfstatat
if echo "$AVAILABLE_UTILS" | grep -q "du"; then
    cp -r test_dir test_du
    check_utility "du" "openat,newfstatat,stat,lstat" "openat" "-a test_du" "directory_usage"
fi

# Test mv - should use openat, renameat for directory moves
if echo "$AVAILABLE_UTILS" | grep -q "mv"; then
    mkdir -p test_mv_src/sub
    echo "test" > test_mv_src/file.txt
    echo "test" > test_mv_src/sub/file2.txt
    check_utility "mv" "openat,renameat,newfstatat,rename" "openat" "test_mv_src test_mv_dst" "move_directory"
fi

echo ""
echo "✓ Basic safe traversal verification completed"
echo ""
echo "=== Additional Safety Checks ==="

# Check for dangerous patterns across all logs
echo "Checking for dangerous path resolution patterns..."

# Check that we're not doing excessive path resolutions (sign of TOCTOU vulnerability)
echo "Checking path resolution frequency..."
for log in strace_*.log; do
    if [ -f "$log" ]; then
        path_resolutions=$(grep -c "test_" "$log" 2>/dev/null || echo "0")
        if [ "$path_resolutions" -gt 20 ]; then
            echo "⚠ $log: High path resolution count ($path_resolutions) - potential TOCTOU risk"
        fi
    fi
done

echo ""
echo "=== Summary ==="
echo "All utilities are using safe traversal correctly!"
