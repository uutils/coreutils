This file documents why some GNU tests are failing:
* cp/cp-a-selinux.sh
* cp/preserve-gid.sh
* date/date-debug.sh
* date/date.pl
* dd/no-allocate.sh
* dd/nocache_eof.sh
* dd/skip-seek-past-file.sh - https://github.com/uutils/coreutils/issues/7216
* dd/stderr.sh
* fmt/non-space.sh
* help/help-version-getopt.sh
* help/help-version.sh
* ls/ls-misc.pl
* ls/stat-free-symlinks.sh
* misc/close-stdout.sh
* misc/nohup.sh
* numfmt/numfmt.pl - https://github.com/uutils/coreutils/issues/7219 / https://github.com/uutils/coreutils/issues/7221
* misc/stdbuf.sh - https://github.com/uutils/coreutils/issues/7072
* misc/tsort.pl - https://github.com/uutils/coreutils/issues/7074
* misc/write-errors.sh
* od/od-float.sh
* ptx/ptx-overrun.sh
* ptx/ptx.pl
* rm/one-file-system.sh - https://github.com/uutils/coreutils/issues/7011
* rm/rm1.sh - https://github.com/uutils/coreutils/issues/9479
* shred/shred-passes.sh
* sort/sort-continue.sh
* sort/sort-debug-keys.sh
* sort/sort-debug-warn.sh
* sort/sort-float.sh
* sort/sort-h-thousands-sep.sh
* sort/sort-merge-fdlimit.sh
* sort/sort-month.sh
* sort/sort.pl
* tac/tac-2-nonseekable.sh
* tail/end-of-device.sh
* tail/follow-stdin.sh
* tail/inotify-rotate-resources.sh
* tail/symlink.sh
* stty/stty-row-col.sh
* stty/stty.sh
* tty/tty-eof.pl
