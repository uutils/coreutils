// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! A collection of procedural macros for the uutests integration test harness.

#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::Item;

/// Ignores an integration test under the WASI test runner, keyed by one of a fixed set of
/// named, de-duplicated reasons.
///
/// Centralizing the reasons keeps the set of known WASI limitations in one place rather than
/// scattered as ad hoc string literals across the test suite. The `wasi_runner` cfg is set
/// only by the WASI CI job (see `.github/workflows/wasi.yml`). Applies to a `#[test]` function
/// or to a module, where the reason attaches to every test within it.
#[proc_macro_attribute]
pub fn wasi_ignore(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let key = args.to_string();
    let reason = match key.as_str() {
        "AbsoluteHostTmpdirOutsideGuestRoot" => {
            "WASI sandbox: absolute host tmpdir path is outside the mapped guest root, so cp cannot stat it"
        }
        "AbsoluteSymlinkTargetsUnfollowable" => {
            "WASI sandbox: absolute symlink targets cannot be followed"
        }
        "AclXattrPreservationFails" => {
            "WASI: ACL/xattr syscalls are not implemented, so attribute preservation (--archive, -p) always fails"
        }
        "AddressSpaceLimitUnsuitable" => {
            "WASI runner target is not suitable for this address-space-limit regression test"
        }
        "CannotDetectUnsafeOverwrite" => "WASI: cannot detect unsafe overwrite",
        "CannotExecShellFromDeletedCwd" => {
            "WASI: wasmtime cannot exec the binary via a raw shell from a deleted cwd"
        }
        "ClosedStdoutGenericIoError" => {
            "WASI: closed stdout reports a generic I/O error rather than BrokenPipe"
        }
        "DebugReportsUnsupported" => {
            "WASI: --debug reports 'unsupported' instead of 'unknown' for this platform"
        }
        "DevNullNotSeekable" => "WASI sandbox: /dev/null is not a seekable device",
        "DirectoryModesUmaskNotFaithful" => {
            "WASI: directory modes/umask are not faithfully reproduced"
        }
        "DotDotCanonicalizationDiffers" => {
            "WASI sandbox: relative '..' path canonicalization differs, causing a false self-copy detection"
        }
        "EmptyPathResolvesToDirectory" => {
            "WASI: an empty path argument resolves to a directory instead of ENOENT"
        }
        "ErrnoMessageMismatch" => "WASI: errno/error-message mismatches",
        "GuestRootWritablePreopen" => {
            "WASI: guest root is a writable preopen, not the protected system root"
        }
        "HostPathsNotVisible" => {
            "WASI sandbox: host paths (/dev, /private, /proc, /sys, etc.) not visible"
        }
        "KillDiscardsUnflushedOutput" => {
            "WASI: killing the wasmtime process discards the unflushed output buffer, so the streamed bytes never reach stdout"
        }
        "LinkInSymlinkedDirectoryDenied" => {
            "WASI sandbox: creating a link inside a symlinked directory is denied"
        }
        "NoBlockSizeSupport" => "WASI: no block size support",
        "NoFifo" => "WASI: no FIFO/mkfifo support",
        "NoLocaleData" => {
            "WASI: no locale data; every locale (even one requested via LC_ALL or other locale env vars) behaves as C"
        }
        "NoPermissionBits" => {
            "WASI: no permission bits/chmod syscall, so file mode cannot be restricted or observed, and explicit mode/ownership preservation always fails"
        }
        "NoPipeSignalSupport" => "WASI: no pipe/signal support",
        "NoRlimitSetrlimitSupport" => "WASI: rlimit/setrlimit not supported",
        "NoStdoutFileRedirection" => "WASI: no stdout-to-file redirection",
        "NoSubprocessSpawning" => "WASI: no subprocess spawning",
        "NoTzdb" => {
            "WASI: no tzdb; TZ env var is not honoured, so timezone-dependent behaviour (DST validation, timestamps) differs from the host"
        }
        "NoUidGid" => "WASI: no uid/gid",
        "NoUnixDomainSockets" => "WASI: no Unix domain socket support",
        "NonUtf8ArgsUnsupported" => {
            "WASI: argv, env values, and paths must be valid UTF-8; wasmtime cannot pass non-UTF-8 data through the sandboxed binary or the spawned test harness"
        }
        "PathCreateDirectoryNoModeParam" => {
            "WASI: path_create_directory has no mode parameter and chmod returns ENOSYS, so directories can't be created with restricted permissions"
        }
        "PermissionErrorsSurfaceAsEnoent" => {
            "WASI: filesystem permission errors surface as ENOENT rather than EACCES"
        }
        "PreEpochTimestampsUnrepresentable" => {
            "WASI: pre-epoch timestamps not representable by path_filestat_set_times"
        }
        "ReadLinkAbsoluteFailsViaHarness" => {
            "WASI: read_link on absolute paths fails under wasmtime via spawned test harness"
        }
        "RenameNotAtomicReplace" => {
            "WASI: path rename is not guaranteed to be an atomic replace, so a concurrent creator can win the race during a forced relink"
        }
        "StdinPositionNotPreserved" => "WASI: stdin file position not preserved through wasmtime",
        "StdoutDashUnsupported" => "WASI: touch - (stdout) is unsupported",
        "SymlinkHardlinkCaps" => {
            "WASI sandbox: symlink/hardlink capability restrictions cause dangling-symlink/same-file detection to differ"
        }
        "SymlinkLoopNoEloop" => {
            "WASI: symlink loop traversal does not surface ELOOP ('Too many levels of symbolic links')"
        }
        "TailFollowDisabled" => "WASI: tail follow mode disabled",
        "Usize32BitOverflow" => "WASI: usize is 32-bit, the host usize::MAX does not parse",
        other => {
            let span = args
                .into_iter()
                .next()
                .map_or_else(proc_macro2::Span::call_site, |token| token.span());
            let message = format!(
                "wasi_ignore: unknown reason key `{other}` (add it to tests/uutests_procs/src/lib.rs)"
            );
            let item = proc_macro2::TokenStream::from(item);
            return TokenStream::from(quote_spanned! { span =>
                compile_error!(#message);
                #item
            });
        }
    };

    let input = syn::parse_macro_input!(item as Item);
    let output = match input {
        Item::Fn(_) | Item::Mod(_) => apply_to_tests(input, reason),
        other => {
            let message = "wasi_ignore can only be applied to a #[test] function or a module containing tests";
            return TokenStream::from(quote_spanned! { proc_macro2::Span::call_site() =>
                compile_error!(#message);
                #other
            });
        }
    };
    TokenStream::from(quote! { #output })
}

/// Attaches the ignore reason to `item`.
///
/// A function receives the attribute directly. A module recurses into its items and
/// attaches the attribute only to functions that look like tests, so plain helper
/// functions stay enabled under the WASI runner.
fn apply_to_tests(item: Item, reason: &str) -> Item {
    match item {
        Item::Fn(mut func) => {
            add_ignore_attr(&mut func, reason);
            Item::Fn(func)
        }
        Item::Mod(module) => Item::Mod(apply_to_module(module, reason)),
        other => other,
    }
}

/// Recurses into a module and attaches the ignore reason to its test functions.
///
/// The filter keeps plain helper functions runnable while still reaching nested modules.
fn apply_to_module(mut module: syn::ItemMod, reason: &str) -> syn::ItemMod {
    if let Some((brace, items)) = module.content.take() {
        let items = items
            .into_iter()
            .map(|it| match it {
                Item::Fn(mut func) => {
                    if is_test_fn(&func) {
                        add_ignore_attr(&mut func, reason);
                    }
                    Item::Fn(func)
                }
                Item::Mod(nested) => Item::Mod(apply_to_module(nested, reason)),
                other => other,
            })
            .collect();
        module.content = Some((brace, items));
    }
    module
}

/// Appends the WASI ignore attribute to a function.
///
/// The attribute is appended last so that under `#[rstest]` it follows the `#[case]`
/// arguments and applies to every generated case, not only the first.
fn add_ignore_attr(func: &mut syn::ItemFn, reason: &str) {
    func.attrs
        .push(syn::parse_quote! { #[cfg_attr(wasi_runner, ignore = #reason)] });
}

/// Reports whether a function carries a `#[test]` or `#[rstest]` attribute.
///
/// The check matches on the last path segment so fully-qualified forms like
/// `#[rstest::rstest]` are recognized.
fn is_test_fn(func: &syn::ItemFn) -> bool {
    func.attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "test" || seg.ident == "rstest")
    })
}

#[cfg(test)]
mod tests {
    use super::apply_to_tests;
    use quote::quote;
    use syn::{Item, parse_quote};

    #[test]
    fn module_annotates_only_tests_and_recurses() {
        let input: Item = parse_quote! {
            mod linux_only {
                #[test]
                fn a_test() {}

                #[rstest]
                fn an_rstest() {}

                fn helper() {}

                mod nested {
                    #[test]
                    fn nested_test() {}

                    fn nested_helper() {}
                }
            }
        };

        let output = apply_to_tests(input, "some reason");
        let rendered = quote!(#output).to_string();

        assert_eq!(rendered.matches("ignore = \"some reason\"").count(), 3);
    }
}
