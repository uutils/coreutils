#!/bin/bash
# Build reusable initramfs for rootfs tests (run once)
# spell-checker:ignore rootfs libm libpthread libdl librt libgcc sysfs tmpfs poweroff
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
ROOTFS_DIR="$REPO_DIR/target/rootfs-test"

echo "Building test rootfs in $ROOTFS_DIR..."

rm -rf "$ROOTFS_DIR"
mkdir -p "$ROOTFS_DIR"/{bin,lib64,proc,sys,dev,tmp,etc,gnu}

# Get busybox
BUSYBOX=/tmp/busybox
[ -f "$BUSYBOX" ] || curl -sL -o "$BUSYBOX" https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox
chmod +x "$BUSYBOX"
cp "$BUSYBOX" "$ROOTFS_DIR/bin/"
cd "$ROOTFS_DIR/bin" && "$BUSYBOX" --list | xargs -I{} ln -sf busybox {} 2>/dev/null

# Copy libs
cp /lib64/{ld-linux-x86-64.so.2,libc.so.6,libm.so.6,libpthread.so.0,libdl.so.2,librt.so.1,libgcc_s.so.1} "$ROOTFS_DIR/lib64/" 2>/dev/null || \
cp /lib/x86_64-linux-gnu/{ld-linux-x86-64.so.2,libc.so.6,libm.so.6,libpthread.so.0,libdl.so.2,librt.so.1,libgcc_s.so.1} "$ROOTFS_DIR/lib64/"

# Copy entire GNU tests directory
cp -r "$REPO_DIR/../gnu/tests" "$ROOTFS_DIR/gnu/"

# Create /etc/mtab placeholder (needed by busybox at startup)
touch "$ROOTFS_DIR/etc/mtab"

# Create init script
cat > "$ROOTFS_DIR/init" << 'INIT'
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sys /sys
ln -sf /proc/mounts /etc/mtab
mkdir -p /tmp && mount -t tmpfs tmpfs /tmp
export PATH="/bin:$PATH" srcdir="/gnu"
cd /gnu/tests
sh "$TEST_SCRIPT"
echo "EXIT:$?"
poweroff -f
INIT
chmod +x "$ROOTFS_DIR/init"

echo "Done. Base rootfs created at $ROOTFS_DIR"
echo "Run tests with: util/run-gnu-test-rootfs.sh tests/df/skip-rootfs.sh"
