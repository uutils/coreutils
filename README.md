uutils coreutils
================

uutils is an attempt at writing universal (as in cross-platform) CLI
utils in [Rust](http://rust-lang.org). This repo is to aggregate the GNU
coreutils rewrites.

Why?
----

Many GNU, linux and other utils are pretty awesome, and obviously
[some](http://gnuwin32.sourceforge.net) [effort](http://unxutils.sourceforge.net)
has been spent in the past to port them to windows. However those projects
are either old, abandonned, hosted on CVS, written in platform-specific C, etc.

Rust provides a good platform-agnostic way of writing systems utils that are easy
to compile anywhere, and this is as good a way as any to try and learn it.

To do
-----

- base64
- basename
- cat
- chcon
- chgrp
- chmod
- chown-core
- chown
- chroot
- cksum
- comm
- copy
- cp-hash
- cp
- csplit
- cut
- date
- dd
- df
- dircolors
- dirname
- du
- echo
- env
- expand
- expr
- extent-scan
- factor
- find-mount-point
- fmt
- fold
- getlimits
- group-list
- groups
- head
- hostid
- hostname
- id
- install
- join
- kill
- lbracket
- libstdbuf
- link
- ln
- logname
- ls-dir
- ls-ls
- ls-vdir
- ls
- make-prime-list
- md5sum
- mkdir
- mkfifo
- mknod
- mktemp
- mv
- nice
- nl
- nohup
- nproc
- numfmt
- od
- operand2sig
- paste
- pathchk
- pinky
- pr
- printf
- prog-fprintf
- ptx
- pwd
- readlink
- realpath
- relpath
- remove
- rm
- rmdir
- runcon
- seq
- setuidgid
- shred
- shuf
- sleep
- sort
- split
- stat
- stdbuf
- stty
- sum
- sync
- tac-pipe
- tac
- tail
- tee
- test
- timeout
- touch
- tr
- truncate
- tsort
- tty
- uname-arch
- uname-uname
- uname
- unexpand
- uniq
- unlink
- uptime
- users
- wc
- who
- whoami

License
-------

uutils are licensed under the MIT License - see the `LICENSE` file for details
