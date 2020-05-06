extern crate proc_macro;

// spell-checker:ignore () SIGPIPE uucore uumain uutils

//## rust proc-macro background info
//* ref: <https://dev.to/naufraghi/procedural-macro-in-rust-101-k3f> @@ <http://archive.is/Vbr5e>
//* ref: [path construction from LitStr](https://oschwald.github.io/maxminddb-rust/syn/struct.LitStr.html) @@ <http://archive.is/8YDua>

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

// main!( EXPR )
// generates a `main()` function for utilities within the uutils group
// EXPR == syn::Expr::Lit::String | syn::Expr::Path::Ident ~ EXPR contains the location of the utility `uumain()` function
//* for future use of "eager" macros and more generic use, EXPR may be in either STRING or IDENT form
//* for ease-of-use, the trailing "::uumain" is optional and will be added if needed
#[proc_macro]
pub fn main(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Tokens { expr } = syn::parse_macro_input!(stream as Tokens);
    // match EXPR as a string literal or an ident path, o/w panic!()
    let expr = match expr {
        syn::Expr::Lit(expr) => match expr.lit {
            syn::Lit::Str(ref lit) => {
                let mut s = lit.value();
                if !s.ends_with("::uumain") {
                    s += "::uumain";
                }
                syn::LitStr::new(&s, proc_macro2::Span::call_site())
                    .parse()
                    .unwrap()
            }
            _ => panic!(),
        },
        syn::Expr::Path(expr) => {
            if &expr.path.segments.last().unwrap().ident.to_string() != "uumain" {
                syn::parse_quote!( #expr::uumain )
            } else {
                expr
            }
        }
        _ => panic!(),
    };
    let f = quote::quote! { #expr(uucore::args().collect()) };
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
