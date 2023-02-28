// Copyright (C) ~ Roy Ivy III <rivy.dev@gmail.com>; MIT license
// spell-checker:ignore backticks

extern crate proc_macro;
use std::{fs::File, io::Read, path::PathBuf};

use proc_macro::{Literal, TokenStream, TokenTree};
use quote::quote;

const MARKDOWN_CODE_FENCES: &str = "```";

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
    let text: String = parse_about(&read_help(&filename));
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
    let text: String = parse_usage(&read_help(&filename));
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
    let text = parse_help_section(&section, &read_help(&filename));
    let rendered = render_markdown(&text);
    TokenTree::Literal(Literal::string(&rendered)).into()
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

/// Get a single section from content
///
/// The section must be a second level section (i.e. start with `##`).
fn parse_help_section(section: &str, content: &str) -> String {
    fn is_section_header(line: &str, section: &str) -> bool {
        line.strip_prefix("##")
            .map_or(false, |l| l.trim().to_lowercase() == section)
    }

    let section = &section.to_lowercase();

    // We cannot distinguish between an empty or non-existing section below,
    // so we do a quick test to check whether the section exists to provide
    // a nice error message.
    if content.lines().all(|l| !is_section_header(l, section)) {
        panic!(
            "The section '{section}' could not be found in the help file. Maybe it is spelled wrong?"
        )
    }

    content
        .lines()
        .skip_while(|&l| !is_section_header(l, section))
        .skip(1)
        .take_while(|l| !l.starts_with("##"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Parses the first markdown code block into a usage string
///
/// The code fences are removed and the name of the util is replaced
/// with `{}` so that it can be replaced with the appropriate name
/// at runtime.
fn parse_usage(content: &str) -> String {
    content
        .lines()
        .skip_while(|l| !l.starts_with(MARKDOWN_CODE_FENCES))
        .skip(1)
        .take_while(|l| !l.starts_with(MARKDOWN_CODE_FENCES))
        .map(|l| {
            // Replace the util name (assumed to be the first word) with "{}"
            // to be replaced with the runtime value later.
            if let Some((_util, args)) = l.split_once(' ') {
                format!("{{}} {args}\n")
            } else {
                "{}\n".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string()
}

/// Parses the text between the first markdown code block and the next header, if any,
/// into an about string.
fn parse_about(content: &str) -> String {
    content
        .lines()
        .skip_while(|l| !l.starts_with(MARKDOWN_CODE_FENCES))
        .skip(1)
        .skip_while(|l| !l.starts_with(MARKDOWN_CODE_FENCES))
        .skip(1)
        .take_while(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_about, parse_help_section, parse_usage};

    #[test]
    fn section_parsing() {
        let input = "\
            # ls\n\
            ## some section\n\
            This is some section\n\
            \n\
            ## ANOTHER SECTION
            This is the other section\n\
            with multiple lines\n";

        assert_eq!(
            parse_help_section("some section", input),
            "This is some section"
        );
        assert_eq!(
            parse_help_section("SOME SECTION", input),
            "This is some section"
        );
        assert_eq!(
            parse_help_section("another section", input),
            "This is the other section\nwith multiple lines"
        );
    }

    #[test]
    #[should_panic]
    fn section_parsing_panic() {
        let input = "\
            # ls\n\
            ## some section\n\
            This is some section\n\
            \n\
            ## ANOTHER SECTION
            This is the other section\n\
            with multiple lines\n";
        parse_help_section("non-existent section", input);
    }

    #[test]
    fn usage_parsing() {
        let input = "\
            # ls\n\
            ```\n\
            ls -l\n\
            ```\n\
            ## some section\n\
            This is some section\n\
            \n\
            ## ANOTHER SECTION
            This is the other section\n\
            with multiple lines\n";

        assert_eq!(parse_usage(input), "{} -l");
    }

    #[test]
    fn multi_line_usage_parsing() {
        let input = "\
            # ls\n\
            ```\n\
            ls -a\n\
            ls -b\n\
            ls -c\n\
            ```\n\
            ## some section\n\
            This is some section\n";

        assert_eq!(parse_usage(input), "{} -a\n{} -b\n{} -c");
    }

    #[test]
    fn about_parsing() {
        let input = "\
            # ls\n\
            ```\n\
            ls -l\n\
            ```\n\
            \n\
            This is the about section\n\
            \n\
            ## some section\n\
            This is some section\n";

        assert_eq!(parse_about(input), "This is the about section");
    }

    #[test]
    fn multi_line_about_parsing() {
        let input = "\
            # ls\n\
            ```\n\
            ls -l\n\
            ```\n\
            \n\
            about a\n\
            \n\
            about b\n\
            \n\
            ## some section\n\
            This is some section\n";

        assert_eq!(parse_about(input), "about a\n\nabout b");
    }
}
