#!/bin/bash
# Run all GNU SMACK tests in QEMU for CI
# Usage: run-gnu-tests-smack-ci.sh [GNU_DIR] [OUTPUT_DIR]
# Outputs logs compatible with gnu-json-result.py
# spell-checker:ignore rootfs cpio newc nographic smackfs devtmpfs tmpfs setuidgid poweroff
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
GNU_DIR="${1:-$REPO_DIR/../gnu}"
OUTPUT_DIR="${2:-$REPO_DIR/target/smack-test-results}"
SMACK_DIR="$REPO_DIR/target/smack-test"

echo "Running GNU SMACK tests..."
echo "  GNU_DIR: $GNU_DIR"
echo "  OUTPUT_DIR: $OUTPUT_DIR"

# Always rebuild SMACK environment to ensure fresh state
echo "Building SMACK test environment..."
rm -rf "$SMACK_DIR"
bash "$SCRIPT_DIR/build-test-smack.sh"

# Verify environment was built
if [ ! -d "$SMACK_DIR/rootfs" ]; then
    echo "Error: SMACK rootfs not found at $SMACK_DIR/rootfs"
    ls -la "$SMACK_DIR" 2>&1 || echo "SMACK_DIR does not exist"
    exit 1
fi

# Debug: show what's in the rootfs
echo "=== ROOTFS CONTENTS ==="
echo "Libraries:"
ls -la "$SMACK_DIR/rootfs/lib64/" || echo "No lib64 directory"
echo "Binaries (first 20):"
ls -la "$SMACK_DIR/rootfs/bin/" | head -20
echo "=== END ROOTFS CONTENTS ==="

# Build all utilities needed for SMACK tests
echo "Building utilities with SMACK support..."
UTILS="ls id mkdir mknod mkfifo"
for U in $UTILS; do
    cargo build --release --manifest-path="$REPO_DIR/Cargo.toml" --package "uu_$U" --bin "$U" --features "uu_$U/smack"
done

# Find all SMACK tests
SMACK_TESTS=$(grep -l 'require_smack_' -r "$GNU_DIR/tests/" 2>/dev/null || true)
if [ -z "$SMACK_TESTS" ]; then
    echo "No SMACK tests found"
    exit 0
fi

echo "Found SMACK tests:"
echo "$SMACK_TESTS"

# Create output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Run each test
for TEST_PATH in $SMACK_TESTS; do
    # Get relative path from tests/ dir (e.g., id/smack.sh)
    TEST_REL="${TEST_PATH#$GNU_DIR/tests/}"
    TEST_DIR=$(dirname "$TEST_REL")
    TEST_NAME=$(basename "$TEST_REL" .sh)

    echo "Running: $TEST_REL"

    # Determine if test needs non-root
    RUN_AS_USER=""
    if echo "$TEST_REL" | grep -q "no-root"; then
        RUN_AS_USER="nobody"
    fi

    # Create working copy
    WORK="/tmp/smack-test-$$"
    rm -rf "$WORK" "$WORK.gz"
    cp -a "$SMACK_DIR/rootfs" "$WORK"

    # Copy built utilities
    for U in $UTILS; do
        rm -f "$WORK/bin/$U"
        cp "$REPO_DIR/target/release/$U" "$WORK/bin/$U"
    done

    # Update init with test script path
    sed -i "s|\$TEST_SCRIPT|$TEST_REL|g" "$WORK/init"

    # Set user for non-root tests
    if [ -n "$RUN_AS_USER" ]; then
        sed -i "s|\$RUN_AS_USER|$RUN_AS_USER|g" "$WORK/init"
    else
        sed -i "s|\$RUN_AS_USER||g" "$WORK/init"
    fi

    # Build initramfs
    (cd "$WORK" && find . | cpio -o -H newc 2>/dev/null | gzip > "$WORK.gz")

    # Run in QEMU
    OUTPUT=$(timeout 120 qemu-system-x86_64 \
        -kernel "$SMACK_DIR/kernel/vmlinuz" \
        -initrd "$WORK.gz" \
        -append "console=ttyS0 quiet panic=-1 security=smack lsm=smack" \
        -nographic -m 256M -no-reboot 2>&1) || true

    # Determine result
    if echo "$OUTPUT" | grep -q "EXIT:0"; then
        RESULT="PASS"
        EXIT_STATUS=0
    elif echo "$OUTPUT" | grep -q "EXIT:77"; then
        RESULT="SKIP"
        EXIT_STATUS=77
    else
        RESULT="FAIL"
        EXIT_STATUS=1
    fi

    echo "  $RESULT: $TEST_REL"

    # Show output for failed or skipped tests
    if [ "$RESULT" = "FAIL" ] || [ "$RESULT" = "SKIP" ]; then
        echo "=== QEMU OUTPUT FOR $TEST_REL ==="
        echo "$OUTPUT" | tail -80
        echo "=== END OUTPUT ==="
    fi

    # Create log file in format expected by gnu-json-result.py
    mkdir -p "$OUTPUT_DIR/$TEST_DIR"
    {
        echo "$OUTPUT"
        echo ""
        echo "$RESULT $TEST_NAME.sh (exit status: $EXIT_STATUS)"
    } > "$OUTPUT_DIR/$TEST_DIR/$TEST_NAME.log"

    # Cleanup
    rm -rf "$WORK" "$WORK.gz"
done

echo "Done. Results in $OUTPUT_DIR"
