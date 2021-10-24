// Copyright (C) ~ Roy Ivy III <rivy.dev@gmail.com>; MIT license

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{self, parse_macro_input, ItemFn};

//## rust proc-macro background info
//* ref: <https://dev.to/naufraghi/procedural-macro-in-rust-101-k3f> @@ <http://archive.is/Vbr5e>
//* ref: [path construction from LitStr](https://oschwald.github.io/maxminddb-rust/syn/struct.LitStr.html) @@ <http://archive.is/8YDua>

//## proc_dbg macro
//* used to help debug the compile-time proc_macro code

#[cfg(feature = "debug")]
macro_rules! proc_dbg {
    ($x:expr) => {
        dbg!($x)
    };
}
#[cfg(not(feature = "debug"))]
macro_rules! proc_dbg {
    ($x:expr) => {};
}

//## main!()

// main!( EXPR )
// generates a `main()` function for utilities within the uutils group
// EXPR == syn::Expr::Lit::String | syn::Expr::Path::Ident ~ EXPR contains the lexical path to the utility `uumain()` function
//* NOTE: EXPR is ultimately expected to be a multi-segment lexical path (eg, `crate::func`); so, if a single segment path is provided, a trailing "::uumain" is automatically added
//* for more generic use (and future use of "eager" macros), EXPR may be in either STRING or IDENT form

struct Tokens {
    expr: syn::Expr,
}

impl syn::parse::Parse for Tokens {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Tokens {
            expr: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn main(stream: TokenStream) -> TokenStream {
    let Tokens { expr } = syn::parse_macro_input!(stream as Tokens);
    proc_dbg!(&expr);

    const ARG_PANIC_TEXT: &str =
        "expected ident lexical path (or a literal string version) to 'uumain()' as argument";

    // match EXPR as a string literal or an ident path, o/w panic!()
    let mut expr = match expr {
        syn::Expr::Lit(expr_lit) => match expr_lit.lit {
            syn::Lit::Str(ref lit_str) => lit_str.parse::<syn::ExprPath>().unwrap(),
            _ => panic!("{}", ARG_PANIC_TEXT),
        },
        syn::Expr::Path(expr_path) => expr_path,
        _ => panic!("{}", ARG_PANIC_TEXT),
    };
    proc_dbg!(&expr);

    // for a single segment ExprPath argument, add trailing '::uumain' segment
    if expr.path.segments.len() < 2 {
        expr = syn::parse_quote!( #expr::uumain );
    };
    proc_dbg!(&expr);

    let f = quote::quote! { #expr(uucore::args_os()) };
    proc_dbg!(&f);

    // generate a uutils utility `main()` function, tailored for the calling utility
    let result = quote::quote! {
        fn main() {
            use std::io::Write;
            uucore::panic::mute_sigpipe_panic(); // suppress extraneous error output for SIGPIPE failures/panics
            let code = #f; // execute utility code
            std::io::stdout().flush().expect("could not flush stdout"); // (defensively) flush stdout for utility prior to exit; see <https://github.com/rust-lang/rust/issues/23818>
            std::process::exit(code);
        }
    };
    TokenStream::from(result)
}

#[proc_macro_attribute]
pub fn gen_uumain(_args: TokenStream, stream: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(stream as ItemFn);

    // Change the name of the function to "uumain_result" to prevent name-conflicts
    ast.sig.ident = Ident::new("uumain_result", Span::call_site());

    let new = quote!(
        pub fn uumain(args: impl uucore::Args) -> i32 {
            #ast
            let result = uumain_result(args);
            match result {
                Ok(()) => uucore::error::get_exit_code(),
                Err(e) => {
                    let s = format!("{}", e);
                    if s != "" {
                        uucore::show_error!("{}", s);
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
