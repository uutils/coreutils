<!-- spell-checker:ignore epipe readdir restorecon SIGALRM capget bigtime rootfs enotsup -->
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

= this system lacks SMACK support =
* tests/mkdir/smack-root.sh
* tests/mkdir/smack-no-root.sh
* tests/id/smack.sh

= timeout returned 142. SIGALRM not handled? =
* tests/misc/timeout-group.sh

= The Swedish locale with blank thousands separator is unavailable. =
* tests/misc/sort-h-thousands-sep.sh

= not running on GNU/Hurd =
* tests/id/gnu-zero-uids.sh

= no rootfs in mtab =
* tests/df/skip-rootfs.sh

= Disabled. Enabled at GNU coreutils > 9.9 =
* tests/misc/tac-continue.sh
* tests/mkdir/writable-under-readonly.sh
* tests/cp/cp-mv-enotsup-xattr.sh
