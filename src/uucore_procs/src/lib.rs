// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore backticks uuhelp

//! A collection of procedural macros for uutils.
#![deny(missing_docs)]

use std::{fs::File, io::Read, path::PathBuf};

use proc_macro::{Literal, TokenStream, TokenTree};
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

// FIXME: This is currently a stub. We could do much more here and could
// even pull in a full markdown parser to get better results.
/// Render markdown into a format that's easier to read in the terminal.
///
/// For now, all this function does is remove backticks.
/// Some ideas for future improvement:
/// - Render headings as bold
/// - Convert triple backticks to indented
/// - Printing tables in a nice format
fn render_markdown(s: &str) -> String {
    s.replace('`', "")
}

/// Get the about text from the help file.
///
/// The about text is assumed to be the text between the first markdown
/// code block and the next header, if any. It may span multiple lines.
#[proc_macro]
pub fn help_about(input: TokenStream) -> TokenStream {
    let input: Vec<TokenTree> = input.into_iter().collect();
    let filename = get_argument(&input, 0, "filename");
    let text: String = uuhelp_parser::parse_about(&read_help(&filename));
    if text.is_empty() {
        panic!("About text not found! Make sure the markdown format is correct");
    }
    TokenTree::Literal(Literal::string(&text)).into()
}

/// Get the usage from the help file.
///
/// The usage is assumed to be surrounded by markdown code fences. It may span
/// multiple lines. The first word of each line is assumed to be the name of
/// the util and is replaced by "{}" so that the output of this function can be
/// used with `uucore::format_usage`.
#[proc_macro]
pub fn help_usage(input: TokenStream) -> TokenStream {
    let input: Vec<TokenTree> = input.into_iter().collect();
    let filename = get_argument(&input, 0, "filename");
    let text: String = uuhelp_parser::parse_usage(&read_help(&filename));
    if text.is_empty() {
        panic!("Usage text not found! Make sure the markdown format is correct");
    }
    TokenTree::Literal(Literal::string(&text)).into()
}

/// Reads a section from a file of the util as a `str` literal.
///
/// It reads from the file specified as the second argument, relative to the
/// crate root. The contents of this file are read verbatim, without parsing or
/// escaping. The name of the help file should match the name of the util.
/// I.e. numfmt should have a file called `numfmt.md`. By convention, the file
/// should start with a top-level section with the name of the util. The other
/// sections must start with 2 `#` characters. Capitalization of the sections
/// does not matter. Leading and trailing whitespace of each section will be
/// removed.
///
/// Example:
/// ```md
/// # numfmt
/// ## About
/// Convert numbers from/to human-readable strings
///
/// ## Long help
/// This text will be the long help
/// ```
///
/// ```rust,ignore
/// help_section!("about", "numfmt.md");
/// ```
#[proc_macro]
pub fn help_section(input: TokenStream) -> TokenStream {
    let input: Vec<TokenTree> = input.into_iter().collect();
    let section = get_argument(&input, 0, "section");
    let filename = get_argument(&input, 1, "filename");

    if let Some(text) = uuhelp_parser::parse_section(&section, &read_help(&filename)) {
        let rendered = render_markdown(&text);
        TokenTree::Literal(Literal::string(&rendered)).into()
    } else {
        panic!(
            "The section '{section}' could not be found in the help file. Maybe it is spelled wrong?"
        )
    }
}

/// Get an argument from the input vector of `TokenTree`.
///
/// Asserts that the argument is a string literal and returns the string value,
/// otherwise it panics with an error.
fn get_argument(input: &[TokenTree], index: usize, name: &str) -> String {
    // Multiply by two to ignore the `','` in between the arguments
    let string = match &input.get(index * 2) {
        Some(TokenTree::Literal(lit)) => lit.to_string(),
        Some(_) => panic!("Argument {index} should be a string literal."),
        None => panic!("Missing argument at index {index} for {name}"),
    };

    string
        .parse::<String>()
        .unwrap()
        .strip_prefix('"')
        .unwrap()
        .strip_suffix('"')
        .unwrap()
        .to_string()
}

/// Read the help file
fn read_help(filename: &str) -> String {
    let mut content = String::new();

    let mut path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    path.push(filename);

    File::open(path)
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();

    content
}
