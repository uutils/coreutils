# Packaging coreutils

<!-- spell-checker:ignore debuginfo manpages backtraces profdata profraw sysroot rustlib msvc Cprofile DESTDIR -->

> **Note**: This page is intended as a guide for packaging the uutils coreutils
> for package maintainers. Normal users probably do not need to read this. If you
> just want to install the coreutils, look at the
> [installation](installation.md) instructions.

The maintainers of this project do not have the capacity to maintain packages
for every distribution and package manager out there. Therefore, we encourage
other people to package the uutils coreutils for their preferred distributions.
You do not need to ask permission for this and you can do this however you want
as long as you comply with the license. However, we do like to hear and
advertise where the uutils coreutils are available, so please do let us know!

## License

The uutils coreutils are licensed under the MIT license. See the
[LICENSE](https://github.com/uutils/coreutils/blob/main/LICENSE) for the full
license text. Make sure to add attribution and the license text to the package
to comply with the license.

## Package

We recommend to name the package `uutils-coreutils`. Just `uutils` is incorrect,
because that is the name of the organization, which also includes other
projects.

## Selecting the utils to include

Not all utils are available on all platforms. To get the full set of utils for a
particular platform, you must enable the feature flag corresponding to the platform name.
For example, on Unix-like system, use `--features unix` and `--features windows`
on Windows.

For a more fine-grained selection, you can enable just the features with the
name of the utils you want to include and disable the default feature set.

Additionally, support for SELinux must be explicitly enabled with the
`feat_selinux` feature.

We recommend including all the utilities that a platform supports.

## Compilation parameters

There are several compile-time flags that allow you to tune the coreutils to
your particular needs. Some distributions, for example, might choose to
minimize the binary size as much as possible.

This can be achieved by customizing the configuration passed to cargo. You can
view the full documentation in the
[cargo documentation](https://doc.rust-lang.org/cargo/reference/profiles.html).

We provide three release profiles out of the box, though you may want to tweak
them:

- `release`: The profile with all performance optimization enabled.
- `release-small`: Optimize binary size.

They include panic abort which removes stack traces on old rust [https://blog.rust-lang.org/2025/12/11/Rust-1.92.0/].
For the precise definition of these profiles, you can look at the root
[`Cargo.toml`](https://github.com/uutils/coreutils/blob/main/Cargo.toml).

The profiles above are just examples. We encourage package maintainers to decide
for themselves what the best parameters for their distribution are. For example,
a distribution focused on embedded systems would probably choose
`release-small`, but another distribution focused on security might enable
bounds checks.

It is also possible to split the debuginfo into a separate package. See the
[`split-debuginfo`](https://doc.rust-lang.org/cargo/reference/profiles.html#split-debuginfo)
option in `cargo`.

## Profile-Guided Optimization (PGO)

The release binaries we publish are built with [Profile-Guided
Optimization](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
on every target whose build machine can run the binary it produces: Linux
x86_64 and aarch64, macOS x86_64 and arm64, and Windows x86_64 and aarch64
(msvc). Packagers are encouraged to do the same: it costs nothing but build
time and needs no source change.

`util/build-pgo.sh` drives the whole process. It runs on Linux, macOS and
Windows:

```bash
rustup component add llvm-tools   # provides llvm-profdata
./util/build-pgo.sh --features unix
```

It performs the four usual PGO steps:

1. build an instrumented multicall binary (`-Cprofile-generate`),
2. run a set of representative workloads (sort, wc, cat, cut, hashing, cp/mv/ls,
   ...) against a corpus it generates itself,
3. merge the raw profiles with `llvm-profdata`,
4. rebuild with `-Cprofile-use`, producing
   `target/coreutils-pgo/<target>/release/coreutils[.exe]`.

Useful options:

- `--features LIST` — feature set to build with (defaults to `unix`); pass the
  same value you use for the real package build (`feat_os_windows` on Windows).
- `--target TRIPLE` — target to build for; defaults to the host. The
  instrumented binary still has to run on the build machine (see below).
- `--target-dir DIR` — where the instrumented build, the corpus and the merged
  profile go (defaults to `target/coreutils-pgo`).
- `--train-only` — stop after step 3, so you can feed the profile to your own
  build command. This is what our CI does:

  ```bash
  ./util/build-pgo.sh --features unix --train-only
  export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Cprofile-use=$(cat target/coreutils-pgo/profdata-path.txt)"
  cargo build --release --features unix
  ```

  Use this if your build system already owns the final `cargo build` (extra
  rustflags, a different profile, a `DESTDIR` install step, ...).
  `profdata-path.txt` holds the absolute path of the merged profile, spelled the
  way the native toolchain expects it — on Windows that matters, since git-bash
  paths (`/d/a/...`) mean nothing to `rustc`.
- `--llvm-profdata PATH` — explicit path to `llvm-profdata`.

Things to know before wiring it into a package build:

- **`llvm-profdata` must match the toolchain that instrumented the binary.** By
  default the script picks the one from the active `rustc` sysroot
  (`rustup component add llvm-tools`). A distribution `llvm-profdata` from a
  different LLVM major version will fail to read the profiles; point at the
  matching one with `--llvm-profdata`.
- **Training runs the freshly built binary**, so the build machine must be able
  to execute it. That rules out emulator-less cross-builds (the script checks
  and fails early rather than producing an empty profile); a foreign target only
  works where the host can run it, such as x86_64 on an arm64 macOS with
  Rosetta. For a true cross-build, either skip PGO or train on the target
  architecture and carry the `.profdata` over.
- **The corpus is generated from scratch** (no reading of `/etc/passwd`,
  `/usr/share/dict/words`, ...) so the profile does not depend on the contents
  of the build machine, and it uses no external tools beyond bash itself. The
  workloads do read this checkout's `src/uu` tree for the `cp`/`ls`/`du`
  training.
- **For reproducible builds, freeze the profile.** The merged `.profdata` is an
  input to the final build, so re-training on another machine can change the
  resulting code layout. Generate it once, ship it as a source artifact, and
  build with `-Cprofile-use=<that file>` instead of re-running the training.
- **Train with the same `lto` and `codegen-units` as the final build.** This is
  the easy way to get a profile that makes things *slower*. Inlining happens
  before instrumentation, so a training build with different settings records
  counters for a call graph the final build no longer has. Our `[profile.release]`
  uses `lto = "fat"` and `codegen-units = 1`, and the script trains with them; if
  you use `--train-only` and then run your own `cargo build`, keep those two
  values identical. When they did not match, `wc -w` came out **57% slower** than
  a plain non-PGO release build, while the profile itself looked perfectly
  healthy.
- **A bad profile fails the build rather than silently degrading it**: the
  script refuses to continue if the merged profile covers fewer than 500
  functions, which is what an environment where the training workloads did not
  actually run looks like. Note that this only catches a profile that is
  *missing*, not one that is *mismatched*: the mismatch above produced a profile
  covering more functions (4098) than the correct one (1883), since a non-LTO
  build still has all the symbols that whole-program codegen later merges away.
  If you change the training or build settings, measure the result.
- It costs a second full build plus the training run, so expect the package
  build to take noticeably longer. On a 24-core x86_64 machine the full script
  takes about 3 minutes for the `unix` feature set, and the resulting binary is
  ~0.8% larger.

## Additional artifacts

This project supports automatically generating manpages and shell completion
files which you may want to include in the package. See the page on
[building from source](build.md) for how to generate these.
