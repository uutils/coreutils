uutils coreutils [![Build Status](https://api.travis-ci.org/uutils/coreutils.svg?branch=master)](https://travis-ci.org/uutils/coreutils)
================

uutils is an attempt at writing universal (as in cross-platform) CLI
utils in [Rust](http://rust-lang.org). This repo is to aggregate the GNU
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

To build all but a few of the available utilities:
```
make DONT_BUILD='UTILITY_1 UTILITY_2'
```

To build only a few of the available utilities:
```
make BUILD='UTILITY_1 UTILITY_2'
```

Installation Instructions
-------------------------

To install all available utilities:
```
make install
```

To install all but a few of the available utilities:
```
make DONT_INSTALL='UTILITY_1 UTILITY_2' install
```

To install only a few of the available utilities:
```
make INSTALL='UTILITY_1 UTILITY_2' install
```

To install every program with a prefix:
```
make PROG_PREFIX=PREFIX_GOES_HERE install
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

Contributions are very welcome, and should target Rust's master branch until
Rust 1.0 is released. You may *claim* an item on the to-do list by following
these steps:

1. Open an issue named "Implement [the utility of your choice]", e.g. "Implement ls"
2. State that you are working on this utility.
3. Develop the utility.
4. Add the reference to your utility into uutils/uutils.rs (required for multibinary).
5. Remove utility from the to-do list on this README.
6. Submit a pull request and close the issue.

The steps above imply that, before starting to work on a utility, you should search the issues to make sure no one else is working on it.

To do
-----

- chcon
- chgrp
- chmod
- chown
- copy
- cp (not much done)
- csplit
- date
- dd
- df
- dircolors
- expand (in progress)
- expr
- getlimits
- install
- join
- libstdbuf
- ln
- ls
- mknod
- mktemp
- mv
- nice
- nproc
- numfmt
- od
- pathchk
- pinky
- pr
- printf
- ptx
- readlink
- realpath
- relpath
- remove
- runcon
- setuidgid
- shred
- shuf
- sort
- split
- stat
- stdbuf
- stty
- tail (not all features implemented)
- test
- timeout
- tsort
- unexpand
- uniq (in progress)
- who

License
-------

uutils is licensed under the MIT License - see the `LICENSE` file for details
