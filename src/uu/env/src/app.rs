use clap::{crate_description, crate_version, App, AppSettings, Arg};

// spell-checker:ignore (ToDO) chdir subcommands

const USAGE: &str = "env [OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]";
const AFTER_HELP: &str = "\
A mere - implies -i. If no COMMAND, print the resulting environment.
";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(crate_description!())
        .usage(USAGE)
        .after_help(AFTER_HELP)
        .setting(AppSettings::AllowExternalSubcommands)
        .arg(Arg::with_name("ignore-environment")
            .short("i")
            .long("ignore-environment")
            .help("start with an empty environment"))
        .arg(Arg::with_name("chdir")
            .short("c")
            .long("chdir")
            .takes_value(true)
            .number_of_values(1)
            .value_name("DIR")
            .help("change working directory to DIR"))
        .arg(Arg::with_name("null")
            .short("0")
            .long("null")
            .help("end each output line with a 0 byte rather than a newline (only valid when \
                    printing the environment)"))
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .takes_value(true)
            .number_of_values(1)
            .value_name("PATH")
            .multiple(true)
            .help("read and set variables from a \".env\"-style configuration file (prior to any \
                    unset and/or set)"))
        .arg(Arg::with_name("unset")
            .short("u")
            .long("unset")
            .takes_value(true)
            .number_of_values(1)
            .value_name("NAME")
            .multiple(true)
            .help("remove variable from the environment"))
}
