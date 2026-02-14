// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore SIGSEGV

//! A collection of procedural macros for uutils.
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;

//## rust proc-macro background info
//* ref: <https://dev.to/naufraghi/procedural-macro-in-rust-101-k3f> @@ <http://archive.is/Vbr5e>
//* ref: [path construction from LitStr](https://oschwald.github.io/maxminddb-rust/syn/struct.LitStr.html) @@ <http://archive.is/8YDua>

/// A procedural macro to define the main function of a uutils binary.
///
/// This macro handles:
/// - SIGPIPE state capture at process startup (before Rust runtime overrides it)
/// - SIGPIPE restoration to default if parent didn't explicitly ignore it
/// - Disabling Rust signal handlers for proper core dumps
/// - Error handling and exit code management
#[proc_macro_attribute]
pub fn main(_args: TokenStream, stream: TokenStream) -> TokenStream {
    let stream = proc_macro2::TokenStream::from(stream);

    let new = quote!(
        // Initialize SIGPIPE state capture at process startup (Unix only).
        // This must be at module level to set up the .init_array static that runs
        // before main() to capture whether SIGPIPE was ignored by the parent process.
        #[cfg(unix)]
        uucore::init_startup_state_capture!();

        pub fn uumain(args: impl uucore::Args) -> i32 {
            #stream

            // Restore SIGPIPE to default if it wasn't explicitly ignored by parent.
            // The Rust runtime ignores SIGPIPE, but we need to respect the parent's
            // signal disposition for proper pipeline behavior (GNU compatibility).
            #[cfg(unix)]
            if !uucore::signals::sigpipe_was_ignored() {
                let _ = uucore::signals::enable_pipe_errors();
            }

            // disable rust signal handlers (otherwise processes don't dump core after e.g. one SIGSEGV)
            #[cfg(unix)]
            uucore::disable_rust_signal_handlers().expect("Disabling rust signal handlers failed");
            let result = uumain(args);
            match result {
                Ok(()) => uucore::error::get_exit_code(),
                Err(e) => {
                    let s = format!("{e}");
                    if s != "" {
                        uucore::show_error!("{s}");
                    }
                    if e.usage() {
                        use std::io::{stderr, Write as _};
                        let _ = writeln!(stderr(),"Try '{} --help' for more information.", uucore::execution_phrase());
                    }
                    e.code()
                }
            }
        }
    );

    TokenStream::from(new)
}
