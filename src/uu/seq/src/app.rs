use clap::{crate_version, App, AppSettings, Arg};

const ABOUT: &str = "Display numbers from FIRST to LAST, in steps of INCREMENT.";

pub const OPT_SEPARATOR: &str = "separator";
pub const OPT_TERMINATOR: &str = "terminator";
pub const OPT_WIDTHS: &str = "widths";

pub const ARG_NUMBERS: &str = "numbers";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .setting(AppSettings::AllowLeadingHyphen)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_SEPARATOR)
                .short("s")
                .long("separator")
                .help("Separator character (defaults to \\n)")
                .takes_value(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name(OPT_TERMINATOR)
                .short("t")
                .long("terminator")
                .help("Terminator character (defaults to \\n)")
                .takes_value(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name(OPT_WIDTHS)
                .short("w")
                .long("widths")
                .help("Equalize widths of all numbers by padding with zeros"),
        )
        .arg(
            Arg::with_name(ARG_NUMBERS)
                .multiple(true)
                .takes_value(true)
                .allow_hyphen_values(true)
                .max_values(3)
                .required(true),
        )
}
