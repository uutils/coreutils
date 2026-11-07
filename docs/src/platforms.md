# Platform support

<!-- markdownlint-disable MD033 MD060 -->
<!-- spell-checker:ignore logind -->

uutils aims to be as "universal" as possible, meaning that we try to support
many platforms. However, it is infeasible for us to guarantee that every
platform works. Just like Rust itself, we therefore have multiple tiers of
platform support, with different guarantees. We support two tiers of platforms:

 - **Tier 1**: All applicable utils are compiled and tested in CI for these
   platforms.
 - **Tier 2**: These platforms are supported but not actively tested. We do accept
   fixes for these platforms.

> **Note**: The tiers are dictated by our CI. We would happily accept a job
> in the CI for testing more platforms, bumping those platforms to tier 1.

## Platforms per tier

The platforms in tier 1 and the platforms that we test in CI are listed below.

| Operating system | Tested targets           |
| ---------------- | ------------------------ |
| **Linux**        | `x86_64-unknown-linux-gnu` <br> `x86_64-unknown-linux-musl` <br> `arm-unknown-linux-gnueabihf` <br> `i686-unknown-linux-gnu` <br> `aarch64-unknown-linux-gnu` |
| **macOS**        | `x86_64-apple-darwin` <br> `aarch64-apple-darwin` |
| **Windows**      | `i686-pc-windows-msvc` <br> `x86_64-pc-windows-gnu` <br> `x86_64-pc-windows-msvc` |
| **FreeBSD**      | `x86_64-unknown-freebsd` |
| **OpenBSD**      | `x86_64-unknown-openbsd` |
| **Android**      | `x86_64-linux-android`     |
| **wasm32**      | `wasm32-wasip1` <br> `wasm32-wasip2` |

The platforms in tier 2 are more vague, but include:

 - untested variations of the platforms above,
 - Redox OS,
 - and BSDs such as NetBSD & DragonFlyBSD.

## Utility compatibility per platform

Not all utils work on every platform. For instance, `chgrp` is not supported on
Windows, because Windows does not have the concept of groups. Below is a full table
detailing which utilities are supported for the tier 1 platforms.

Note that for some utilities, not all functionality is supported on each
platform. This is documented per utility.

{{ #include platform_table.md }}

## Linux traditional login records

The traditional Linux reader opens utmp without waiting for FIFO writers,
checks that the opened file is regular, and reads from that same file descriptor.
Replacing the path with a FIFO after opening therefore cannot redirect its reads.
Waiting for record locks has a shared ten-second budget per iterator. Exhausting
that budget ends iteration without reading unlocked data. As with other utmp
read errors, the iterator does not report an error separately from end of file.
The selected path is retained across successive iterators.
With the `systemd_logind` feature enabled, the default path selects systemd;
custom paths select traditional files, including for subsequent iterators.
Selecting the default path explicitly switches back to systemd.
