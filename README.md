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

Build Instructions
------------------

To simply build all available utilities:
```
make
```

To build all but a few of the available utilities:
```
make DONT_BUILD='UTILITY_1 UTILITY_2'
```

To build only a few of the available utilities:
```
make BUILD='UTILITY_1 UTILITY_2'
```

Test Instructions
-----------------

To simply test all available utilities:
```
make test
```

To test all but a few of the available utilities:
```
make DONT_TEST='UTILITY_1 UTILITY_2' test
```

To test only a few of the available utilities:
```
make TEST='UTILITY_1 UTILITY_2' test
```

Contribute
----------

Contributions are very welcome, and should target rust's master branch until
rust 1.0 is released. You may *claim* an item on the to-do list by following
these steps:

1. Open an issue named "Implement [the utility of your choice]", e.g. "Implement ls"
2. State that you are working on this utility.
3. Develop the utility.
4. Submit a pull request and close the issue.  Your pull request should include deleting the utility from the to-do list on this README.

The steps above imply that, before starting to work on a utility, you should search the issues to make sure no one else is working on it.

To do
-----

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
- du
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
- head ( in progress )
- hostid
- hostname (in progress)
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
- readlink
- realpath
- relpath
- remove
- runcon
- seq
- setuidgid
- shred
- shuf
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
- uniq (in progress)
- unlink
- uptime
- users
- who

License
-------

uutils are licensed under the MIT License - see the `LICENSE` file for details
