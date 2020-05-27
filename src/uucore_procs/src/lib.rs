#![allow(dead_code)] // work-around for GH:rust-lang/rust#62127; maint: can be removed when MinSRV >= v1.38.0
#![allow(unused_macros)] // work-around for GH:rust-lang/rust#62127; maint: can be removed when MinSRV >= v1.38.0

// Copyright (C) ~ Roy Ivy III <rivy.dev@gmail.com>; MIT license

extern crate proc_macro;

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
#[cfg(not(test))] // work-around for GH:rust-lang/rust#62127; maint: can be removed when MinSRV >= v1.38.0
pub fn main(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Tokens { expr } = syn::parse_macro_input!(stream as Tokens);
    proc_dbg!(&expr);

    const ARG_PANIC_TEXT: &str =
        "expected ident lexical path (or a literal string version) to 'uumain()' as argument";

    // match EXPR as a string literal or an ident path, o/w panic!()
    let mut expr = match expr {
        syn::Expr::Lit(expr_lit) => match expr_lit.lit {
            syn::Lit::Str(ref lit_str) => lit_str.parse::<syn::ExprPath>().unwrap(),
            _ => panic!(ARG_PANIC_TEXT),
        },
        syn::Expr::Path(expr_path) => expr_path,
        _ => panic!(ARG_PANIC_TEXT),
    };
    proc_dbg!(&expr);

    // for a single segment ExprPath argument, add trailing '::uumain' segment
    if expr.path.segments.len() < 2 {
        expr = syn::parse_quote!( #expr::uumain );
    };
    proc_dbg!(&expr);

    let f = quote::quote! { #expr(uucore::args().collect()) };
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
    proc_macro::TokenStream::from(result)
}
