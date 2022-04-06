// Copyright (C) ~ Roy Ivy III <rivy.dev@gmail.com>; MIT license

extern crate proc_macro;
use std::{fs::File, io::Read, path::PathBuf};

use proc_macro::{Literal, TokenStream, TokenTree};
use quote::quote;

//## rust proc-macro background info
//* ref: <https://dev.to/naufraghi/procedural-macro-in-rust-101-k3f> @@ <http://archive.is/Vbr5e>
//* ref: [path construction from LitStr](https://oschwald.github.io/maxminddb-rust/syn/struct.LitStr.html) @@ <http://archive.is/8YDua>

#[proc_macro_attribute]
pub fn main(_args: TokenStream, stream: TokenStream) -> TokenStream {
    let stream = proc_macro2::TokenStream::from(stream);

    let new = quote!(
        pub fn uumain(args: impl uucore::Args) -> i32 {
            #stream
            let result = uumain(args);
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

fn parse_help(section: &str) -> String {
    let section = section.to_lowercase();
    let section = section.trim_matches('"');
    let mut content = String::new();
    let mut path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // The package name will be something like uu_numfmt, hence we split once
    // on '_' and take the second element. The help section should then be in a
    // file called numfmt.md
    path.push(format!(
        "{}.md",
        std::env::var("CARGO_PKG_NAME")
            .unwrap()
            .split_once('_')
            .unwrap()
            .1,
    ));

    File::open(path)
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();

    content
        .lines()
        .skip_while(|&l| {
            l.strip_prefix("##")
                .map_or(true, |l| l.trim().to_lowercase() != section)
        })
        .skip(1)
        .take_while(|l| !l.starts_with("##"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Get the usage from the "Usage" section in the help file.
///
/// The usage is assumed to be surrounded by markdown code fences. It may span
/// multiple lines. The first word of each line is assumed to be the name of
/// the util and is replaced by "{}" so that the output of this function can be
/// used with `uucore::format_usage`.
#[proc_macro]
pub fn help_usage(_input: TokenStream) -> TokenStream {
    let text: String = parse_help("usage")
        .strip_suffix("```")
        .unwrap()
        .lines()
        .skip(1) // Skip the "```" of markdown syntax
        .map(|l| {
            // Replace the util name (assumed to be the first word) with "{}"
            // to be replaced with the runtime value later.
            if let Some((_util, args)) = l.split_once(' ') {
                format!("{{}} {}", args)
            } else {
                "{}".to_string()
            }
        })
        .collect();
    TokenTree::Literal(Literal::string(&text)).into()
}

/// Reads a section from the help file of the util as a `str` literal.
///
/// It is read verbatim, without parsing or escaping. The name of the help file
/// should match the name of the util. I.e. numfmt should have a file called
/// `numfmt.md`. By convention, the file should start with a top-level section
/// with the name of the util. The other sections must start with 2 `#`
/// characters. Capitalization of the sections does not matter. Leading and
/// trailing whitespace will be removed. Example:
/// ```md
/// # numfmt
/// ## About
/// Convert numbers from/to human-readable strings
///
/// ## Long help
/// This text will be the long help
/// ```
#[proc_macro]
pub fn help_section(input: TokenStream) -> TokenStream {
    let input: Vec<TokenTree> = input.into_iter().collect();
    let value = match &input.get(0) {
        Some(TokenTree::Literal(literal)) => literal.to_string(),
        _ => panic!("Input to help_section should be a string literal!"),
    };
    let input_str: String = value.parse().unwrap();
    let text = parse_help(&input_str);
    TokenTree::Literal(Literal::string(&text)).into()
}
