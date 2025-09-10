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
#[proc_macro_attribute]
pub fn main(_args: TokenStream, stream: TokenStream) -> TokenStream {
    let stream = proc_macro2::TokenStream::from(stream);

    let new = quote!(
        pub fn uumain(args: impl uucore::Args) -> i32 {
            #stream

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
                        eprintln!("Try '{} --help' for more information.", uucore::execution_phrase());
                    }
                    e.code()
                }
            }
        }
    );

    TokenStream::from(new)
}
