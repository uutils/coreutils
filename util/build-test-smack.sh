#!/bin/bash
# Build reusable initramfs for SMACK tests (run once)
# Downloads Arch Linux kernel with SMACK support
# spell-checker:ignore rootfs zstd unzstd libm libpthread libdl librt libgcc libnss nsswitch sysfs smackfs devtmpfs tmpfs setuidgid poweroff
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
SMACK_DIR="$REPO_DIR/target/smack-test"

echo "Building SMACK test environment in $SMACK_DIR..."

rm -rf "$SMACK_DIR"
mkdir -p "$SMACK_DIR"/{rootfs/{bin,lib64,proc,sys,dev,tmp,etc,gnu},kernel}

# Download Arch Linux kernel (has SMACK built-in)
MIRROR="https://geo.mirror.pkgbuild.com/core/os/x86_64"

if [ ! -f /tmp/arch-vmlinuz ]; then
    echo "Downloading Arch Linux kernel with SMACK..."
    # Find current kernel package name dynamically
    KERNEL_PKG=$(curl -sL "$MIRROR/" | grep -oP 'linux-[0-9][^"]*-x86_64\.pkg\.tar\.zst' | grep -v headers | sort -V | tail -1)
    # Alternative: use a specific known-good version
    # KERNEL_PKG="linux-6.12.6.arch1-1-x86_64.pkg.tar.zst"
    if [ -z "$KERNEL_PKG" ]; then
        echo "Error: Could not find Arch Linux kernel package"
        exit 1
    fi
    echo "Found kernel package: $KERNEL_PKG"
    echo "Kernel version: $(echo $KERNEL_PKG | grep -oP 'linux-\K[0-9]+\.[0-9]+\.[0-9]+')"
    curl -sL -o /tmp/arch-kernel.pkg.tar.zst "$MIRROR/$KERNEL_PKG"
    zstd -d /tmp/arch-kernel.pkg.tar.zst -o /tmp/arch-kernel.pkg.tar 2>/dev/null || \
        unzstd /tmp/arch-kernel.pkg.tar.zst -o /tmp/arch-kernel.pkg.tar
    # Extract kernel - list contents first to find the path
    VMLINUZ_PATH=$(tar -tf /tmp/arch-kernel.pkg.tar | grep 'vmlinuz$' | head -1)
    if [ -z "$VMLINUZ_PATH" ]; then
        echo "Error: Could not find vmlinuz in kernel package"
        tar -tf /tmp/arch-kernel.pkg.tar | head -20
        exit 1
    fi
    echo "Extracting: $VMLINUZ_PATH"
    tar -xf /tmp/arch-kernel.pkg.tar -C /tmp "$VMLINUZ_PATH"
    mv "/tmp/$VMLINUZ_PATH" /tmp/arch-vmlinuz
    rm -rf /tmp/usr /tmp/arch-kernel.pkg.tar /tmp/arch-kernel.pkg.tar.zst
fi
cp /tmp/arch-vmlinuz "$SMACK_DIR/kernel/vmlinuz"

# Get busybox
BUSYBOX=/tmp/busybox
[ -f "$BUSYBOX" ] || curl -sL -o "$BUSYBOX" https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox
chmod +x "$BUSYBOX"
cp "$BUSYBOX" "$SMACK_DIR/rootfs/bin/"
cd "$SMACK_DIR/rootfs/bin" && "$BUSYBOX" --list | xargs -I{} ln -sf busybox {}
echo "Busybox applets created:"
ls -la "$SMACK_DIR/rootfs/bin/" | head -20

# Copy required libraries using ldd on a test binary to find them
echo "Finding and copying required libraries..."

# Use ldconfig to find libraries
copy_lib_by_name() {
    local lib="$1"
    local path=$(ldconfig -p | grep "$lib" | head -1 | awk '{print $NF}')
    if [ -n "$path" ] && [ -f "$path" ]; then
        cp -L "$path" "$SMACK_DIR/rootfs/lib64/"
        echo "  Copied: $path"
    else
        echo "  Warning: $lib not found via ldconfig"
        # Fallback: search common paths
        for dir in /lib64 /lib/x86_64-linux-gnu /usr/lib64 /usr/lib/x86_64-linux-gnu /usr/lib; do
            for f in "$dir"/$lib "$dir"/$lib.*; do
                if [ -f "$f" ]; then
                    cp -L "$f" "$SMACK_DIR/rootfs/lib64/"
                    echo "  Copied: $f"
                    return
                fi
            done
        done
        echo "  ERROR: $lib not found anywhere"
    fi
}

# Essential libraries
for lib in ld-linux-x86-64.so.2 libc.so.6 libm.so.6 libgcc_s.so.1 libpthread.so.0 libdl.so.2 librt.so.1 libnss_files.so.2; do
    copy_lib_by_name "$lib"
done

echo "Libraries in rootfs:"
ls -la "$SMACK_DIR/rootfs/lib64/"

# Create symlink for library path compatibility
mkdir -p "$SMACK_DIR/rootfs/lib"
ln -sf /lib64 "$SMACK_DIR/rootfs/lib/x86_64-linux-gnu"

# Create /etc/nsswitch.conf for NSS
echo "passwd: files
group: files
shadow: files" > "$SMACK_DIR/rootfs/etc/nsswitch.conf"

# Copy entire GNU tests directory
cp -r "$REPO_DIR/../gnu/tests" "$SMACK_DIR/rootfs/gnu/"
cp "$REPO_DIR/../gnu/init.cfg" "$SMACK_DIR/rootfs/gnu/" 2>/dev/null || true

# Create /etc/mtab placeholder
touch "$SMACK_DIR/rootfs/etc/mtab"

# Create minimal /etc/passwd and /etc/group (include nobody user for non-root tests)
cat > "$SMACK_DIR/rootfs/etc/passwd" << 'PASSWD'
root:x:0:0:root:/root:/bin/sh
nobody:x:65534:65534:nobody:/nonexistent:/bin/sh
PASSWD
cat > "$SMACK_DIR/rootfs/etc/group" << 'GROUP'
root:x:0:
nobody:x:65534:
GROUP

# Create init script (supports $RUN_AS_USER to run as non-root)
cat > "$SMACK_DIR/rootfs/init" << 'INIT'
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sys /sys
mount -t smackfs smackfs /sys/fs/smackfs 2>/dev/null || true

# Set up permissive SMACK policy - allow all access for testing
if [ -d /sys/fs/smackfs ]; then
    # Set current process to floor label
    echo "_" > /proc/self/attr/current 2>/dev/null || true
    # Allow floor label to access everything
    echo "_ _ rwxat" > /sys/fs/smackfs/load 2>/dev/null || true
    echo "_ * rwxat" > /sys/fs/smackfs/load 2>/dev/null || true
    echo "* _ rwxat" > /sys/fs/smackfs/load 2>/dev/null || true
fi

mount -t devtmpfs devtmpfs /dev 2>/dev/null || {
    # Fallback: create minimal device nodes
    mknod /dev/null c 1 3 2>/dev/null || true
    mknod /dev/zero c 1 5 2>/dev/null || true
    mknod /dev/tty c 5 0 2>/dev/null || true
    chmod 666 /dev/null /dev/zero /dev/tty 2>/dev/null || true
}
ln -sf /proc/mounts /etc/mtab
mkdir -p /tmp && mount -t tmpfs tmpfs /tmp
chmod 1777 /tmp
export PATH="/bin:$PATH" srcdir="/gnu" LD_LIBRARY_PATH="/lib64:/lib/x86_64-linux-gnu"
export built_programs="id ls mkdir mknod mkfifo"

# Debug: Check if SMACK is enabled
echo "=== SMACK STATUS ==="
cat /proc/filesystems | grep smack || echo "smackfs not in /proc/filesystems"
ls -la /sys/fs/smackfs 2>&1 || echo "/sys/fs/smackfs not available"
cat /proc/self/attr/current 2>&1 || echo "Cannot read SMACK label"
echo "=== END SMACK STATUS ==="

if [ -n "$RUN_AS_USER" ]; then
    # Create writable test directory for non-root user
    mkdir -p /tmp/test-work/tests
    chmod -R 777 /tmp/test-work
    cd /tmp/test-work/tests
    # Copy test files to writable location (preserving structure)
    cp -r /gnu/tests/* /tmp/test-work/tests/ 2>/dev/null || true
    cp /gnu/init.cfg /tmp/test-work/ 2>/dev/null || true
    chmod -R 777 /tmp/test-work
    export srcdir="/tmp/test-work"
    # Run as non-root user using setuidgid (from busybox)
    setuidgid $RUN_AS_USER sh "$TEST_SCRIPT"
else
    cd /gnu/tests
    sh "$TEST_SCRIPT"
fi
echo "EXIT:$?"
poweroff -f
INIT
chmod +x "$SMACK_DIR/rootfs/init"

echo "Done. SMACK test environment created at $SMACK_DIR"
echo "Run tests with: util/run-gnu-test-smack.sh tests/id/smack.sh"
