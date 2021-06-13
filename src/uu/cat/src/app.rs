use clap::{crate_version, App, Arg};

// spell-checker:ignore (ToDO) nonprint nonblank nonprinting

const NAME: &str = "cat";
const SYNTAX: &str = "[OPTION]... [FILE]...";
const SUMMARY: &str = "Concatenate FILE(s), or standard input, to standard output
 With no FILE, or when FILE is -, read standard input.";

pub mod options {
    pub const FILE: &str = "file";
    pub const SHOW_ALL: &str = "show-all";
    pub const NUMBER_NONBLANK: &str = "number-nonblank";
    pub const SHOW_NONPRINTING_ENDS: &str = "e";
    pub const SHOW_ENDS: &str = "show-ends";
    pub const NUMBER: &str = "number";
    pub const SQUEEZE_BLANK: &str = "squeeze-blank";
    pub const SHOW_NONPRINTING_TABS: &str = "t";
    pub const SHOW_TABS: &str = "show-tabs";
    pub const SHOW_NONPRINTING: &str = "show-nonprinting";
}
pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .name(NAME)
        .version(crate_version!())
        .usage(SYNTAX)
        .about(SUMMARY)
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
        .arg(
            Arg::with_name(options::SHOW_ALL)
                .short("A")
                .long(options::SHOW_ALL)
                .help("equivalent to -vET"),
        )
        .arg(
            Arg::with_name(options::NUMBER_NONBLANK)
                .short("b")
                .long(options::NUMBER_NONBLANK)
                .help("number nonempty output lines, overrides -n")
                .overrides_with(options::NUMBER),
        )
        .arg(
            Arg::with_name(options::SHOW_NONPRINTING_ENDS)
                .short("e")
                .help("equivalent to -vE"),
        )
        .arg(
            Arg::with_name(options::SHOW_ENDS)
                .short("E")
                .long(options::SHOW_ENDS)
                .help("display $ at end of each line"),
        )
        .arg(
            Arg::with_name(options::NUMBER)
                .short("n")
                .long(options::NUMBER)
                .help("number all output lines"),
        )
        .arg(
            Arg::with_name(options::SQUEEZE_BLANK)
                .short("s")
                .long(options::SQUEEZE_BLANK)
                .help("suppress repeated empty output lines"),
        )
        .arg(
            Arg::with_name(options::SHOW_NONPRINTING_TABS)
                .short("t")
                .long(options::SHOW_NONPRINTING_TABS)
                .help("equivalent to -vT"),
        )
        .arg(
            Arg::with_name(options::SHOW_TABS)
                .short("T")
                .long(options::SHOW_TABS)
                .help("display TAB characters at ^I"),
        )
        .arg(
            Arg::with_name(options::SHOW_NONPRINTING)
                .short("v")
                .long(options::SHOW_NONPRINTING)
                .help("use ^ and M- notation, except for LF (\\n) and TAB (\\t)"),
        )
}
