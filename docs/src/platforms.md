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

## Linux login records

Library users can enable `uucore`'s `utmpx` feature alone for traditional records.
The separate `systemd-logind` feature adds Linux logind support and its runtime
and D-Bus dependencies. The four utilities below enable both features.

`who`, `pinky`, `users`, and `uptime` prefer the default utmp file when it is a
readable regular file containing at least one complete record. This checks file
availability, not record validity: all-zero records and files containing only
boot records still take precedence over systemd-logind. Explicit non-default
utmp paths always use the traditional reader.

Otherwise these utilities try libsystemd at runtime and return to the
traditional reader if loading or session enumeration fails.
The traditional Linux reader opens utmp without waiting for FIFO writers,
checks that the opened file is regular, and reads from that same file descriptor.
Replacing the path with a FIFO after opening therefore cannot redirect its reads.
Waiting for record locks has a shared ten-second budget per iterator. Exhausting
that budget ends iteration without reading unlocked data. As with other utmp
read errors, the iterator does not report an error separately from end of file.
The selected path is retained across successive iterators. Selecting the default
path again restores the default file/logind selection rules.

Before systemd 254, session timestamps and leader PIDs are obtained from the
[public logind D-Bus interface](https://www.freedesktop.org/software/systemd/man/252/org.freedesktop.login1.html).
If neither source can provide a value, `who`
and `pinky` display `?` for the unavailable timestamp; `who -u` also displays
`?` for an unavailable PID. These sessions still count toward the user total.
Callers can select session metadata with `utmpx::ReadOptions`; its default
requests both timestamps and PIDs. `ReadOptions::BASIC` omits those session
fields while retaining the boot timestamp. `users`, `who -q`, and `uptime` use
this mode, so they do not query D-Bus. Other listings request only the fields
they display, and only missing values are queried.

The D-Bus fallback has a shared one-second wait budget across all sessions,
including connection startup. It uses zbus with cancellable asynchronous I/O;
a timeout drops the pending connection and requests. Later calls can retry
without waiting for an earlier query to finish. Session enumeration and values
already obtained through sd-login remain available.

Both traditional and logind timestamps use the local UTC offset at the record's
time, including the daylight saving rules in effect then.

The public `UtmpxRecord::pid()` and `login_time()` accessors retain their original
return types, using zero and the Unix epoch for unavailable logind values.
Callers displaying these values should use `try_pid()` and `try_login_time()`
to distinguish missing data, as the utilities do.

Static musl builds cannot load libsystemd dynamically. They retain musl's utmp
stub behavior, so login listings can be empty even on a system running logind.
