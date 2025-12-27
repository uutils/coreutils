#!/bin/bash
# Run GNU test in QEMU with rootfs visible
# Usage: run-gnu-test-rootfs.sh tests/df/skip-rootfs.sh
# spell-checker:ignore rootfs cpio newc nographic
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
ROOTFS_DIR="$REPO_DIR/target/rootfs-test"
WORK=/tmp/rootfs-test-$$

[ -z "$1" ] && { echo "Usage: $0 tests/<util>/<test>.sh"; exit 1; }
TEST_SCRIPT="${1#tests/}"  # Strip leading tests/ since we cd to /gnu/tests

# Build base rootfs if needed
[ -d "$ROOTFS_DIR" ] || bash "$SCRIPT_DIR/build-test-rootfs.sh"

# Get utility name from test path (e.g., df/skip-rootfs.sh -> df)
UTIL=$(echo "$TEST_SCRIPT" | cut -d/ -f1)

# Build the utility
cargo build --manifest-path="$REPO_DIR/Cargo.toml" --package "uu_$UTIL" --bin "$UTIL"

# Create working copy
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT
cp -a "$ROOTFS_DIR" "$WORK"

# Copy built utility (remove busybox symlink first)
rm -f "$WORK/bin/$UTIL"
cp "$REPO_DIR/target/debug/$UTIL" "$WORK/bin/$UTIL"

# Update init with test script path
sed -i "s|\$TEST_SCRIPT|$TEST_SCRIPT|g" "$WORK/init"

# Build initramfs
cd "$WORK" && find . | cpio -o -H newc 2>/dev/null | gzip > "$WORK.gz"

# Find kernel
KERNEL=$(ls /boot/vmlinuz-* 2>/dev/null | head -1)
[ -z "$KERNEL" ] && { echo "No kernel found"; exit 1; }

# Run test
OUTPUT=$(timeout 60 qemu-system-x86_64 -kernel "$KERNEL" -initrd "$WORK.gz" \
    -append "console=ttyS0 quiet panic=-1" -nographic -m 256M -no-reboot 2>&1)

rm -f "$WORK.gz"
echo "$OUTPUT" | tail -20

# Check result
if echo "$OUTPUT" | grep -q "EXIT:0"; then
    echo "PASS: $TEST_SCRIPT"
    exit 0
elif echo "$OUTPUT" | grep -q "EXIT:77"; then
    echo "SKIP: $TEST_SCRIPT"
    exit 77
else
    echo "FAIL: $TEST_SCRIPT"
    exit 1
fi
