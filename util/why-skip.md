# spell-checker:ignore epipe readdir restorecon SIGALRM capget bigtime rootfs enotsup

= skipped test: breakpoint not hit =
* tests/tail-2/inotify-race2.sh
* tail-2/inotify-race.sh

= internal test failure: maybe LD_PRELOAD doesn't work? =
* tests/rm/rm-readdir-fail.sh
* tests/rm/r-root.sh
* tests/df/skip-duplicates.sh
* tests/df/no-mtab-status.sh

= LD_PRELOAD was ineffective? =
* tests/cp/nfs-removal-race.sh

= failed to create hfs file system =
* tests/mv/hardlink-case.sh

= temporarily disabled =
* tests/mkdir/writable-under-readonly.sh

= this system lacks SMACK support =
* tests/mkdir/smack-root.sh
* tests/mkdir/smack-no-root.sh
* tests/id/smack.sh

= this system lacks SELinux support =
* tests/mkdir/selinux.sh
* tests/mkdir/restorecon.sh
* tests/misc/selinux.sh
* tests/misc/chcon.sh
* tests/misc/invalid-opt
* tests/runcon/runcon-no-reorder.sh
* tests/install/install-Z-selinux.sh
* tests/install/install-C-selinux.sh
* tests/id/no-context.sh
* tests/id/context.sh
* tests/cp/no-ctx.sh
* tests/cp/cp-a-selinux.sh

= failed to set xattr of file =
* tests/misc/xattr.sh

= timeout returned 142. SIGALRM not handled? =
* tests/misc/timeout-group.sh

= FULL_PARTITION_TMPDIR not defined =
* tests/misc/tac-continue.sh

= can't get window size =
* tests/misc/stty-row-col.sh

= The Swedish locale with blank thousands separator is unavailable. =
* tests/misc/sort-h-thousands-sep.sh

= this shell lacks ulimit support =
* tests/misc/csplit-heap.sh

= multicall binary is disabled =
* tests/misc/coreutils.sh

= not running on GNU/Hurd =
* tests/id/gnu-zero-uids.sh

= file system cannot represent big timestamps =
* tests/du/bigtime.sh

= no rootfs in mtab =
* tests/df/skip-rootfs.sh

= insufficient mount/ext2 support =
* tests/df/problematic-chars.sh
* tests/cp/cp-mv-enotsup-xattr.sh

= 512 byte aligned O_DIRECT is not supported on this (file) system =
* tests/dd/direct.sh

= skipped test: /usr/bin/touch -m -d '1998-01-15 23:00' didn't work =
* tests/misc/ls-time.sh

= requires controlling input terminal =
* tests/misc/stty-pairs.sh
* tests/misc/stty.sh
* tests/misc/stty-invalid.sh

= insufficient SEEK_DATA support =
* tests/cp/sparse-perf.sh
* tests/cp/sparse-extents.sh
* tests/cp/sparse-extents-2.sh
* tests/cp/sparse-2.sh
