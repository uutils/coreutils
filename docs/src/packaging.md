# Packaging coreutils

<!-- spell-checker:ignore debuginfo manpages backtraces -->

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
particular platform, you must enable the feature flag with the platform name.
For example, on Unix-like system, use `--features unix` and `--features windows`
on Windows.

For a more fine-grained selection, you can enable just the features with the
name of the utils you want to include and disable the default feature set.

Additionally, support for SELinux must explicitly enabled with the
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

- `release`: This is the standard Rust release profile, but with link-time
  optimization enabled. It is a balance between compile time, performance and a
  reasonable amount of debug info. The main drawback of this profile is that the
  binary is quite large (roughly 2x the GNU coreutils).
- `release-fast`: Every setting is tuned for the best performance, at the cost
  of compile time. This binary is still quite large.
- `release-small`: Generates the smallest binary possible. This strips _all_
  debug info from the binary and leads to worse backtraces. The performance of
  this profile is also really good as it is close to the `release-fast` profile,
  but with all debuginfo stripped.

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

## Additional artifacts

This project supports automatically generating manpages and shell completion
files which you may want to include in the package. See the page on
[building from source](build.md) for how to generate these.
