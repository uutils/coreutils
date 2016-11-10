uutils coreutils
================

[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/coreutils/blob/master/LICENSE)
[![Build Status](https://api.travis-ci.org/uutils/coreutils.svg?branch=master)](https://travis-ci.org/uutils/coreutils)
[![Build status](https://ci.appveyor.com/api/projects/status/787ltcxgy86r20le?svg=true)](https://ci.appveyor.com/project/Arcterus/coreutils)

uutils is an attempt at writing universal (as in cross-platform) CLI
utils in [Rust](http://www.rust-lang.org). This repo is to aggregate the GNU
coreutils rewrites.

Why?
----

Many GNU, Linux and other utils are pretty awesome, and obviously
[some](http://gnuwin32.sourceforge.net) [effort](http://unxutils.sourceforge.net)
has been spent in the past to port them to Windows. However, those projects
are either old, abandoned, hosted on CVS, written in platform-specific C, etc.

Rust provides a good, platform-agnostic way of writing systems utils that are easy
to compile anywhere, and this is as good a way as any to try and learn it.

Build Instructions
------------------

To simply build all available utilities:
```
make
```

(on Windows use [MinGW/MSYS](http://www.mingw.org/wiki/MSYS) or `Cygwin` make and make sure you have `rustc` in `PATH`)

To build all but a few of the available utilities:
```
make SKIP_UTILS='UTILITY_1 UTILITY_2'
```

To build only a few of the available utilities:
```
make UTILS='UTILITY_1 UTILITY_2'
```

Installation Instructions
-------------------------

To install all available utilities:
```
make install
```

To install all but a few of the available utilities:
```
make SKIP_UTILS='UTILITY_1 UTILITY_2' install
```

To install only a few of the available utilities:
```
make UTILS='UTILITY_1 UTILITY_2' install
```

To install every program with a prefix (e.g. uu-echo uu-cat):
```
make PROG_PREFIX=PREFIX_GOES_HERE install
```

To install the multicall binary:
```
make MULTICALL=y install
```

Set install parent directory (default value is /usr/local):
```
make PREFIX=/my/path install
```

Uninstallation Instructions
---------------------------

To uninstall all utilities:
```
make uninstall
```

To uninstall every program with a set prefix:
```
make PROG_PREFIX=PREFIX_GOES_HERE uninstall
```

To uninstall the multicall binary:
```
make MULTICALL=y uninstall
```

To uninstall from a custom parent directory:
```
make PREFIX=/my/path uninstall
```

Test Instructions
-----------------

To simply test all available utilities:
```
make test
```

To test all but a few of the available utilities:
```
make SKIP_UTILS='UTILITY_1 UTILITY_2' test
```

To test only a few of the available utilities:
```
make UTILS='UTILITY_1 UTILITY_2' test
```

To include tests for unimplemented behavior:
```
make UTILS='UTILITY_1 UTILITY_2' SPEC=y test
```

Run busybox tests
-----------------

This testing functionality is only available on *nix operating systems

To run busybox's tests for all utilities for which busybox has tests
```
make busytest
```

To run busybox's tests for a few of the available utilities
```
make UTILS='UTILITY_1 UTILITY_2' busytest
```

To pass an argument like "-v" to the busybox test runtime
```
make UTILS='UTILITY_1 UTILITY_2' RUNTEST_ARGS='-v' busytest
```

Contribute
----------

To contribute to coreutils, please see [CONTRIBUTING](CONTRIBUTING.md).

To do
-----

* [x] arch
* [x] base32
* [x] base64
* [x] basename
* [x] cat
* [ ] chcon
* [x] chgrp
* [x] chmod
* [x] chown
* [x] chroot
* [x] cksum
* [x] comm
* [ ] cp (not much done)
* [ ] csplit
* [x] cut
* [ ] date
* [ ] dd
* [ ] df
* [x] dircolors
* [x] dirname
* [x] du
* [x] echo
* [x] env
* [x] expand
* [ ] expr (almost done, no regular expressions)
* [x] factor
* [x] false
* [x] fmt
* [x] fold
* [x] groups
* [x] hashsum
* [x] head
* [x] hostid
* [x] hostname
* [x] id
* [ ] install (a couple of missing options)
* [ ] join
* [x] kill
* [x] link
* [x] ln
* [x] logname
* [ ] ls
* [x] ~~md5sum~~, ~~sha1sum~~, ~~sha224sum~~, ~~sha256sum~~, ~~sha384sum~~, ~~sha512sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/hashsum/hashsum.rs))
* [x] mkdir
* [x] mkfifo
* [x] mknod
* [x] mktemp
* [x] mv
* [ ] more (in progress, needs lots of work)
* [x] nice
* [x] nl
* [x] nohup
* [x] nproc
* [ ] numfmt
* [ ] od (almost complete, `--strings` and 128-bit datatypes are missing)
* [x] paste
* [x] pathchk
* [x] pinky
* [ ] pr
* [x] printenv
* [ ] printf
* [x] ptx
* [x] pwd
* [x] readlink
* [x] realpath
* [x] relpath
* [x] rm
* [x] rmdir
* [ ] runcon
* [x] seq
* [x] shred
* [x] shuf
* [x] sleep
* [ ] sort (a couple of options implemented)
* [ ] split (a couple of missing options)
* [x] stat
* [x] stdbuf
* [ ] stty
* [x] sum
* [x] sync
* [x] tac
* [ ] tail (not all features implemented)
* [x] tee
* [ ] test (not all features implemented)
* [x] timeout
* [x] touch
* [x] tr
* [x] true
* [x] truncate
* [x] tsort
* [x] tty
* [x] uname
* [x] unexpand
* [x] uniq
* [x] unlink
* [x] uptime
* [x] users
* [x] wc
* [x] who
* [x] whoami
* [x] yes

License
-------

uutils is licensed under the MIT License - see the `LICENSE` file for details
