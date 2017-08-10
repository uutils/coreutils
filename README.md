uutils coreutils
================

[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/coreutils/blob/master/LICENSE)
[![Build Status](https://api.travis-ci.org/uutils/coreutils.svg?branch=master)](https://travis-ci.org/uutils/coreutils)
[![Build status](https://ci.appveyor.com/api/projects/status/787ltcxgy86r20le?svg=true)](https://ci.appveyor.com/project/Arcterus/coreutils)
[![LOC](https://tokei.rs/b1/github/uutils/coreutils?category=code)](https://github.com/Aaronepower/tokei)

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

Utilities
---------

| Done      | Semi-Done | To Do  |
|-----------|-----------|--------|
| arch      | cp        | chcon  |
| base32    | expr (no regular expressions) | csplit |
| base64    | install   | dd     |
| basename  | ls        | df     |
| cat       | more      | join   |
| chgrp     | od (`--strings` and 128-bit data types missing) | numfmt |
| chmod     | printf    | pr     |
| chown     | sort      | runcon |
| chroot    | split     | stty   |
| cksum     | tail      |        |
| comm      | test      |        |
| cut       | date      |        |
| dircolors |           |        |
| dirname   |           |        |
| du        |           |        |
| echo      |           |        |
| env       |           |        |
| expand    |           |        |
| factor    |           |        |
| false     |           |        |
| fmt       |           |        |
| fold      |           |        |
| groups    |           |        |
| hashsum   |           |        |
| head      |           |        |
| hostid    |           |        |
| hostname  |           |        |
| id        |           |        |
| kill      |           |        |
| link      |           |        |
| ln        |           |        |
| logname   |           |        |
| ~~md5sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/hashsum/hashsum.rs)) | |
| ~~sha1sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/hashsum/hashsum.rs)) | |
| ~~sha224sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/hashsum/hashsum.rs)) | |
| ~~sha256sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/hashsum/hashsum.rs)) | |
| ~~sha384sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/hashsum/hashsum.rs)) | |
| ~~sha512sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/hashsum/hashsum.rs)) | |
| mkdir     |           |        |
| mkfifo    |           |        |
| mknod     |           |        |
| mktemp    |           |        |
| mv        |           |        |
| nice      |           |        |
| nl        |           |        |
| nohup     |           |        |
| nproc     |           |        |
| paste     |           |        |
| pathchk   |           |        |
| pinky     |           |        |
| printenv  |           |        |
| ptx       |           |        |
| pwd       |           |        |
| readlink  |           |        |
| realpath  |           |        |
| relpath   |           |        |
| rm        |           |        |
| rmdir     |           |        |
| seq       |           |        |
| shred     |           |        |
| shuf      |           |        |
| sleep     |           |        |
| stat      |           |        |
| stdbuf    |           |        |
| sum       |           |        |
| sync      |           |        |
| tac       |           |        |
| tee       |           |        |
| timeout   |           |        |
| touch     |           |        |
| tr        |           |        |
| true      |           |        |
| truncate  |           |        |
| tsort     |           |        |
| tty       |           |        |
| uname     |           |        |
| unexpand  |           |        |
| uniq      |           |        |
| unlink    |           |        |
| uptime    |           |        |
| users     |           |        |
| wc        |           |        |
| who       |           |        |
| whoami    |           |        |
| yes       |           |        |

License
-------

uutils is licensed under the MIT License - see the `LICENSE` file for details
