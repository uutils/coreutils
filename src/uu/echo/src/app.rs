use clap::{crate_version, App, Arg};

const SUMMARY: &str = "display a line of text";
const USAGE: &str = "[OPTIONS]... [STRING]...";
const AFTER_HELP: &str = r#"
 Echo the STRING(s) to standard output.

 If -e is in effect, the following sequences are recognized:

 \\\\      backslash
 \\a      alert (BEL)
 \\b      backspace
 \\c      produce no further output
 \\e      escape
 \\f      form feed
 \\n      new line
 \\r      carriage return
 \\t      horizontal tab
 \\v      vertical tab
 \\0NNN   byte with octal value NNN (1 to 3 digits)
 \\xHH    byte with hexadecimal value HH (1 to 2 digits)
"#;

pub mod options {
    pub const STRING: &str = "STRING";
    pub const NO_NEWLINE: &str = "no_newline";
    pub const ENABLE_BACKSLASH_ESCAPE: &str = "enable_backslash_escape";
    pub const DISABLE_BACKSLASH_ESCAPE: &str = "disable_backslash_escape";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        // TrailingVarArg specifies the final positional argument is a VarArg
        // and it doesn't attempts the parse any further args.
        // Final argument must have multiple(true) or the usage string equivalent.
        .setting(clap::AppSettings::TrailingVarArg)
        .setting(clap::AppSettings::AllowLeadingHyphen)
        .version(crate_version!())
        .about(SUMMARY)
        .after_help(AFTER_HELP)
        .usage(USAGE)
        .arg(
            Arg::with_name(options::NO_NEWLINE)
                .short("n")
                .help("do not output the trailing newline")
                .takes_value(false)
                .display_order(1),
        )
        .arg(
            Arg::with_name(options::ENABLE_BACKSLASH_ESCAPE)
                .short("e")
                .help("enable interpretation of backslash escapes")
                .takes_value(false)
                .display_order(2),
        )
        .arg(
            Arg::with_name(options::DISABLE_BACKSLASH_ESCAPE)
                .short("E")
                .help("disable interpretation of backslash escapes (default)")
                .takes_value(false)
                .display_order(3),
        )
        .arg(
            Arg::with_name(options::STRING)
                .multiple(true)
                .allow_hyphen_values(true),
        )
}
