use clap::{crate_version, App, Arg};

pub mod options {
    pub const BOURNE_SHELL: &str = "bourne-shell";
    pub const C_SHELL: &str = "c-shell";
    pub const PRINT_DATABASE: &str = "print-database";
    pub const FILE: &str = "FILE";
}

const SUMMARY: &str = "Output commands to set the LS_COLORS environment variable.";
const LONG_HELP: &str = "
 If FILE is specified, read it to determine which colors to use for which
 file types and extensions.  Otherwise, a precompiled database is used.
 For details on the format of these files, run 'dircolors --print-database'
";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(SUMMARY)
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::BOURNE_SHELL)
                .long("sh")
                .short("b")
                .visible_alias("bourne-shell")
                .help("output Bourne shell code to set LS_COLORS")
                .display_order(1),
        )
        .arg(
            Arg::with_name(options::C_SHELL)
                .long("csh")
                .short("c")
                .visible_alias("c-shell")
                .help("output C shell code to set LS_COLORS")
                .display_order(2),
        )
        .arg(
            Arg::with_name(options::PRINT_DATABASE)
                .long("print-database")
                .short("p")
                .help("print the byte counts")
                .display_order(3),
        )
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
}
