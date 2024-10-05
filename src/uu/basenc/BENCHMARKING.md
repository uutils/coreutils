<!--
spell-checker:ignore gibibyte toybox
-->

# Benchmarking base32, base64, and basenc

Note that the functionality of the `base32` and `base64` programs is identical to that of the `basenc` program, using
the "--base32" and "--base64" options, respectively. For that reason, it is only necessary to benchmark `basenc`.

To compare the runtime performance of the uutils implementation with the GNU Core Utilities implementation, you can
use a benchmarking tool like [hyperfine][0].

hyperfine currently does not measure maximum memory usage. Memory usage can be benchmarked using [poop][2], or
[toybox][3]'s "time" subcommand (both are Linux only).

Build the `basenc` binary using the release profile:

```Shell
cargo build --package uu_basenc --profile release
```

## Expected performance

uutils' `basenc` performs streaming decoding and encoding, and therefore should perform all operations with a constant
maximum memory usage, regardless of the size of the input. Release builds currently use less than 3 mebibytes of
memory, and memory usage greater than 10 mebibytes should be considered a bug.

As of September 2024, uutils' `basenc` has runtime performance equal to or superior to GNU Core Utilities' `basenc` in
in most scenarios. uutils' `basenc` uses slightly more memory, but given how small these quantities are in absolute
terms (see above), this is highly unlikely to be practically relevant to users.

## Benchmark results (2024-09-27)

### Setup

```Shell
# Use uutils' dd to create a 1 gibibyte in-memory file filled with random bytes (Linux only).
# On other platforms, you can use /tmp instead of /dev/shm, but note that /tmp is not guaranteed to be in-memory.
coreutils dd if=/dev/urandom of=/dev/shm/one-random-gibibyte bs=1024 count=1048576

# Encode this file for use in decoding performance testing
/usr/bin/basenc --base32hex -- /dev/shm/one-random-gibibyte 1>/dev/shm/one-random-gibibyte-base32hex-encoded
/usr/bin/basenc --z85 -- /dev/shm/one-random-gibibyte 1>/dev/shm/one-random-gibibyte-z85-encoded
```

### Programs being tested

uutils' `basenc`:

```
❯ git rev-list HEAD | coreutils head -n 1 -- -
a0718ef0ffd50539a2e2bc0095c9fadcd70ab857
```

GNU Core Utilities' `basenc`:

```
❯ /usr/bin/basenc --version | coreutils head -n 1 -- -
basenc (GNU coreutils) 9.4
```

### Encoding performance

#### "--base64", default line wrapping (76 characters)

➕ Faster than GNU Core Utilities

```
❯ hyperfine \
    --sort \
    command \
    -- \
    '/usr/bin/basenc --base64 -- /dev/shm/one-random-gibibyte 1>/dev/null' \
    './target/release/basenc --base64 -- /dev/shm/one-random-gibibyte 1>/dev/null'

Benchmark 1: /usr/bin/basenc --base64 -- /dev/shm/one-random-gibibyte 1>/dev/null
  Time (mean ± σ):     965.1 ms ±   7.9 ms    [User: 766.2 ms, System: 193.4 ms]
  Range (min … max):   950.2 ms … 976.9 ms    10 runs

Benchmark 2: ./target/release/basenc --base64 -- /dev/shm/one-random-gibibyte 1>/dev/null
  Time (mean ± σ):     696.6 ms ±   9.1 ms    [User: 574.9 ms, System: 117.3 ms]
  Range (min … max):   683.1 ms … 713.5 ms    10 runs

Relative speed comparison
        1.39 ±  0.02  /usr/bin/basenc --base64 -- /dev/shm/one-random-gibibyte 1>/dev/null
        1.00          ./target/release/basenc --base64 -- /dev/shm/one-random-gibibyte 1>/dev/null
```

#### "--base16", no line wrapping

➖ Slower than GNU Core Utilities

```
❯ poop \
    '/usr/bin/basenc --base16 --wrap 0 -- /dev/shm/one-random-gibibyte' \
    './target/release/basenc --base16 --wrap 0 -- /dev/shm/one-random-gibibyte'

Benchmark 1 (6 runs): /usr/bin/basenc --base16 --wrap 0 -- /dev/shm/one-random-gibibyte
  measurement          mean ± σ            min … max           outliers         delta
  wall_time           836ms ± 13.3ms     822ms …  855ms          0 ( 0%)        0%
  peak_rss           2.05MB ± 73.0KB    1.94MB … 2.12MB          0 ( 0%)        0%
  cpu_cycles         2.85G  ± 32.8M     2.82G  … 2.91G           0 ( 0%)        0%
  instructions       14.0G  ± 58.7      14.0G  … 14.0G           0 ( 0%)        0%
  cache_references   70.0M  ± 6.48M     63.7M  … 78.8M           0 ( 0%)        0%
  cache_misses        582K  ±  172K      354K  …  771K           0 ( 0%)        0%
  branch_misses       667K  ± 4.55K      662K  …  674K           0 ( 0%)        0%
Benchmark 2 (6 runs): ./target/release/basenc --base16 --wrap 0 -- /dev/shm/one-random-gibibyte
  measurement          mean ± σ            min … max           outliers         delta
  wall_time           884ms ± 6.38ms     878ms …  895ms          0 ( 0%)        💩+  5.7% ±  1.6%
  peak_rss           2.65MB ± 66.8KB    2.55MB … 2.74MB          0 ( 0%)        💩+ 29.3% ±  4.4%
  cpu_cycles         3.15G  ± 8.61M     3.14G  … 3.16G           0 ( 0%)        💩+ 10.6% ±  1.1%
  instructions       10.5G  ±  275      10.5G  … 10.5G           0 ( 0%)        ⚡- 24.9% ±  0.0%
  cache_references   93.5M  ± 6.10M     87.2M  …  104M           0 ( 0%)        💩+ 33.7% ± 11.6%
  cache_misses        415K  ± 52.3K      363K  …  474K           0 ( 0%)          - 28.8% ± 28.0%
  branch_misses      1.43M  ± 4.82K     1.42M  … 1.43M           0 ( 0%)        💩+113.9% ±  0.9%
```

### Decoding performance

#### "--base32hex"

➕ Faster than GNU Core Utilities

```
❯ hyperfine \
    --sort \
    command \
    -- \
    '/usr/bin/basenc --base32hex --decode -- /dev/shm/one-random-gibibyte-base32hex-encoded 1>/dev/null' \
    './target/release/basenc --base32hex --decode -- /dev/shm/one-random-gibibyte-base32hex-encoded 1>/dev/null'

Benchmark 1: /usr/bin/basenc --base32hex --decode -- /dev/shm/one-random-gibibyte-base32hex-encoded 1>/dev/null
  Time (mean ± σ):      7.154 s ±  0.082 s    [User: 6.802 s, System: 0.323 s]
  Range (min … max):    7.051 s …  7.297 s    10 runs

Benchmark 2: ./target/release/basenc --base32hex --decode -- /dev/shm/one-random-gibibyte-base32hex-encoded 1>/dev/null
  Time (mean ± σ):      2.679 s ±  0.025 s    [User: 2.446 s, System: 0.221 s]
  Range (min … max):    2.649 s …  2.718 s    10 runs

Relative speed comparison
        2.67 ±  0.04  /usr/bin/basenc --base32hex --decode -- /dev/shm/one-random-gibibyte-base32hex-encoded 1>/dev/null
        1.00          ./target/release/basenc --base32hex --decode -- /dev/shm/one-random-gibibyte-base32hex-encoded 1>/dev/null
```

#### "--z85", with "--ignore-garbage"

➕ Faster than GNU Core Utilities

```
❯ poop \
    '/usr/bin/basenc --decode --ignore-garbage --z85 -- /dev/shm/one-random-gibibyte-z85-encoded' \
    './target/release/basenc --decode --ignore-garbage --z85 -- /dev/shm/one-random-gibibyte-z85-encoded'

Benchmark 1 (3 runs): /usr/bin/basenc --decode --ignore-garbage --z85 -- /dev/shm/one-random-gibibyte-z85-encoded
  measurement          mean ± σ            min … max           outliers         delta
  wall_time          14.4s  ± 68.4ms    14.3s  … 14.4s           0 ( 0%)        0%
  peak_rss           1.98MB ± 10.8KB    1.97MB … 1.99MB          0 ( 0%)        0%
  cpu_cycles         58.4G  ±  211M     58.3G  … 58.7G           0 ( 0%)        0%
  instructions       74.7G  ± 64.0      74.7G  … 74.7G           0 ( 0%)        0%
  cache_references   41.8M  ±  624K     41.2M  … 42.4M           0 ( 0%)        0%
  cache_misses        693K  ±  118K      567K  …  802K           0 ( 0%)        0%
  branch_misses      1.24G  ±  183K     1.24G  … 1.24G           0 ( 0%)        0%
Benchmark 2 (3 runs): ./target/release/basenc --decode --ignore-garbage --z85 -- /dev/shm/one-random-gibibyte-z85-encoded
  measurement          mean ± σ            min … max           outliers         delta
  wall_time          2.80s  ± 17.9ms    2.79s  … 2.82s           0 ( 0%)        ⚡- 80.5% ±  0.8%
  peak_rss           2.61MB ± 67.4KB    2.57MB … 2.69MB          0 ( 0%)        💩+ 31.9% ±  5.5%
  cpu_cycles         10.8G  ± 27.9M     10.8G  … 10.9G           0 ( 0%)        ⚡- 81.5% ±  0.6%
  instructions       39.0G  ±  353      39.0G  … 39.0G           0 ( 0%)        ⚡- 47.7% ±  0.0%
  cache_references    114M  ± 2.43M      112M  …  116M           0 ( 0%)        💩+173.3% ±  9.6%
  cache_misses       1.06M  ±  288K      805K  … 1.37M           0 ( 0%)          + 52.6% ± 72.0%
  branch_misses      1.18M  ± 14.7K     1.16M  … 1.19M           0 ( 0%)        ⚡- 99.9% ±  0.0%
```

[0]: https://github.com/sharkdp/hyperfine
[1]: https://github.com/sharkdp/hyperfine?tab=readme-ov-file#installation
[2]: https://github.com/andrewrk/poop
[3]: https://landley.net/toybox/
