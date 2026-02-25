#!/bin/bash
# Run GNU SMACK/ROOTFS tests in QEMU with SMACK-enabled kernel
# Usage: run-gnu-tests-smack-ci.sh [GNU_DIR] [OUTPUT_DIR]
# spell-checker:ignore rootfs zstd unzstd cpio newc nographic smackfs devtmpfs tmpfs poweroff libm libgcc libpthread libdl librt sysfs rwxat setuidgid
set -e

: ${PROFILE:=release-small}
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
GNU_DIR="${1:-$REPO_DIR/../gnu}"
OUTPUT_DIR="${2:-$REPO_DIR/target/qemu-test-results}"
QEMU_DIR="$REPO_DIR/target/qemu-test"

echo "Setting up SMACK/ROOTFS test environment..."
rm -rf "$QEMU_DIR"
mkdir -p "$QEMU_DIR"/{rootfs/{bin,lib64,proc,sys,dev,tmp,etc,gnu},kernel}

# Download Arch Linux kernel (has SMACK built-in)
if [ ! -f /tmp/arch-vmlinuz ]; then
    echo "Downloading Arch Linux kernel..."
    curl -sL --retry 5 --retry-delay 2 --retry-all-errors \
        -o /tmp/arch-kernel.pkg.tar.zst "https://archlinux.org/packages/core/x86_64/linux/download/"
    zstd -d /tmp/arch-kernel.pkg.tar.zst -o /tmp/arch-kernel.pkg.tar 2>/dev/null || unzstd /tmp/arch-kernel.pkg.tar.zst -o /tmp/arch-kernel.pkg.tar
    VMLINUZ_PATH=$(tar -tf /tmp/arch-kernel.pkg.tar | grep 'vmlinuz$' | head -1)
    tar -xf /tmp/arch-kernel.pkg.tar -C /tmp "$VMLINUZ_PATH"
    mv "/tmp/$VMLINUZ_PATH" /tmp/arch-vmlinuz
    rm -rf /tmp/usr /tmp/arch-kernel.pkg.tar /tmp/arch-kernel.pkg.tar.zst
fi
cp /tmp/arch-vmlinuz "$QEMU_DIR/kernel/vmlinuz"

# Setup busybox
BUSYBOX=/tmp/busybox
[ -f "$BUSYBOX" ] || curl -sL -o "$BUSYBOX" https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox
chmod +x "$BUSYBOX"
cp "$BUSYBOX" "$QEMU_DIR/rootfs/bin/"
(cd "$QEMU_DIR/rootfs/bin" && "$BUSYBOX" --list | xargs -I{} ln -sf busybox {} 2>/dev/null)

# Copy required libraries
for lib in ld-linux-x86-64.so.2 libc.so.6 libm.so.6 libgcc_s.so.1 libpthread.so.0 libdl.so.2 librt.so.1; do
    path=$(ldconfig -p | grep "$lib" | head -1 | awk '{print $NF}')
    [ -n "$path" ] && [ -f "$path" ] && cp -L "$path" "$QEMU_DIR/rootfs/lib64/" 2>/dev/null || true
done

# Create minimal config files
echo -e "root:x:0:0:root:/root:/bin/sh\nnobody:x:65534:65534:nobody:/nonexistent:/bin/sh" > "$QEMU_DIR/rootfs/etc/passwd"
echo -e "root:x:0:\nnobody:x:65534:" > "$QEMU_DIR/rootfs/etc/group"
touch "$QEMU_DIR/rootfs/etc/mtab"

# Copy GNU tests
cp -r "$GNU_DIR/tests" "$QEMU_DIR/rootfs/gnu/"

# Create init script
cat > "$QEMU_DIR/rootfs/init" << 'INIT'
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sys /sys
mount -t smackfs smackfs /sys/fs/smackfs 2>/dev/null || true
if [ -d /sys/fs/smackfs ]; then
    echo "_" > /proc/self/attr/current 2>/dev/null || true
    echo "_ _ rwxat" > /sys/fs/smackfs/load 2>/dev/null || true
fi
mount -t devtmpfs devtmpfs /dev 2>/dev/null || true
ln -sf /proc/mounts /etc/mtab
mkdir -p /tmp && mount -t tmpfs tmpfs /tmp
chmod 1777 /tmp
export PATH="/bin:$PATH" srcdir="/gnu" LD_LIBRARY_PATH="/lib64"
if [ -n "$RUN_AS_USER" ]; then
    # Run in /tmp so non-root user can create temp directories
    cd /tmp
    setuidgid "$RUN_AS_USER" sh "/gnu/tests/$TEST_SCRIPT"
else
    cd /gnu/tests
    sh "$TEST_SCRIPT"
fi
echo "EXIT:$?"
poweroff -f
INIT
chmod +x "$QEMU_DIR/rootfs/init"

# Build utilities for SMACK/ROOTFS tests
echo "Building utilities for SMACK/ROOTFS tests..."
cargo build --profile="${PROFILE}" --features=feat_smack,id,ls,mkdir,mkfifo,mknod,df --no-default-features

# Find SMACK tests and tests requiring rootfs in mtab (only available in QEMU environment)
QEMU_TESTS=$(grep -l -E 'require_smack_|rootfs in mtab' -r "$GNU_DIR/tests/" 2>/dev/null | sort -u || true)
[ -z "$QEMU_TESTS" ] && { echo "No SMACK/ROOTFS tests found"; exit 0; }

echo "Found $(echo "$QEMU_TESTS" | wc -l) SMACK/ROOTFS tests"

# Create output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Run each test
for TEST_PATH in $QEMU_TESTS; do
    TEST_REL="${TEST_PATH#"$GNU_DIR"/tests/}"
    TEST_DIR=$(dirname "$TEST_REL")
    TEST_NAME=$(basename "$TEST_REL" .sh)

    echo "Running: $TEST_REL"

    # Determine if test needs non-root user
    RUN_AS_USER=""
    if echo "$TEST_REL" | grep -q "no-root"; then
        RUN_AS_USER="nobody"
    fi

    # Create working copy
    WORK="/tmp/qemu-test-$$"
    rm -rf "$WORK" "$WORK.gz"
    cp -a "$QEMU_DIR/rootfs" "$WORK"

    # Hardlink utilities for SMACK/ROOTFS tests
    for U in $("$REPO_DIR/target/${PROFILE}/coreutils" --list); do
        ln -vf "$REPO_DIR/target/${PROFILE}/coreutils" "$WORK/bin/$U"
    done

    # Set test script path and user
    sed -i "s|\$TEST_SCRIPT|$TEST_REL|g" "$WORK/init"
    if [ -n "$RUN_AS_USER" ]; then
        sed -i "s|\$RUN_AS_USER|$RUN_AS_USER|g" "$WORK/init"
    else
        sed -i "s|\$RUN_AS_USER||g" "$WORK/init"
    fi

    # Build initramfs and run
    (cd "$WORK" && find . | cpio -o -H newc 2>/dev/null | gzip > "$WORK.gz")

    OUTPUT=$(timeout 120 qemu-system-x86_64 \
        -kernel "$QEMU_DIR/kernel/vmlinuz" \
        -initrd "$WORK.gz" \
        -append "console=ttyS0 quiet panic=-1 security=smack lsm=smack" \
        -nographic -m 256M -no-reboot 2>&1) || true

    # Determine result
    if echo "$OUTPUT" | grep -q "EXIT:0"; then
        RESULT="PASS"; EXIT_STATUS=0
    elif echo "$OUTPUT" | grep -q "EXIT:77"; then
        RESULT="SKIP"; EXIT_STATUS=77
    else
        RESULT="FAIL"; EXIT_STATUS=1
    fi

    echo "  $RESULT: $TEST_REL"

    # Create log file for gnu-json-result.py
    mkdir -p "$OUTPUT_DIR/$TEST_DIR"
    echo "$OUTPUT" > "$OUTPUT_DIR/$TEST_DIR/$TEST_NAME.log"
    echo "" >> "$OUTPUT_DIR/$TEST_DIR/$TEST_NAME.log"
    echo "$RESULT $TEST_NAME.sh (exit status: $EXIT_STATUS)" >> "$OUTPUT_DIR/$TEST_DIR/$TEST_NAME.log"

    rm -rf "$WORK" "$WORK.gz"
done

echo "Done. Results in $OUTPUT_DIR"
