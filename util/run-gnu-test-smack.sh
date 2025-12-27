#!/bin/bash
# Run GNU test in QEMU with SMACK enabled
# Usage: run-gnu-test-smack.sh tests/id/smack.sh
# spell-checker:ignore rootfs cpio newc nographic
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
SMACK_DIR="$REPO_DIR/target/smack-test"
WORK=/tmp/smack-test-$$

[ -z "$1" ] && { echo "Usage: $0 tests/<util>/<test>.sh"; exit 1; }
TEST_SCRIPT="${1#tests/}"  # Strip leading tests/

# Build SMACK environment if needed
[ -d "$SMACK_DIR" ] || bash "$SCRIPT_DIR/build-test-smack.sh"

# Get utility name from test path (e.g., id/smack.sh -> id)
UTIL=$(echo "$TEST_SCRIPT" | cut -d/ -f1)

# Determine which utilities to build based on test
# mkdir smack tests need mkdir, mknod, mkfifo, and ls (for require_smack_ check)
if [ "$UTIL" = "mkdir" ]; then
    UTILS="ls mkdir mknod mkfifo"
else
    UTILS="ls $UTIL"
fi

# Build the utilities with smack feature (release mode for smaller binaries)
for U in $UTILS; do
    cargo build --release --manifest-path="$REPO_DIR/Cargo.toml" --package "uu_$U" --bin "$U" --features "uu_$U/smack"
done

# Create working copy
cleanup() { rm -rf "$WORK" "$WORK.gz"; }
trap cleanup EXIT
cp -a "$SMACK_DIR/rootfs" "$WORK"

# Copy built utilities (remove busybox symlinks first)
for U in $UTILS; do
    rm -f "$WORK/bin/$U"
    cp "$REPO_DIR/target/release/$U" "$WORK/bin/$U"
done

# Update init with test script path
sed -i "s|\$TEST_SCRIPT|$TEST_SCRIPT|g" "$WORK/init"

# For smack-no-root.sh test, run as non-root user
if echo "$TEST_SCRIPT" | grep -q "no-root"; then
    sed -i "s|\$RUN_AS_USER|nobody|g" "$WORK/init"
else
    sed -i "s|\$RUN_AS_USER||g" "$WORK/init"
fi

# Build initramfs
cd "$WORK" && find . | cpio -o -H newc 2>/dev/null | gzip > "$WORK.gz"

# Run test with SMACK-enabled kernel
OUTPUT=$(timeout 60 qemu-system-x86_64 -kernel "$SMACK_DIR/kernel/vmlinuz" -initrd "$WORK.gz" \
    -append "console=ttyS0 quiet panic=-1 security=smack lsm=smack" -nographic -m 256M -no-reboot 2>&1)

echo "$OUTPUT" | tail -20

# Check result
if echo "$OUTPUT" | grep -q "EXIT:0"; then
    echo "PASS: $1"
    exit 0
elif echo "$OUTPUT" | grep -q "EXIT:77"; then
    echo "SKIP: $1"
    exit 77
else
    echo "FAIL: $1"
    exit 1
fi
