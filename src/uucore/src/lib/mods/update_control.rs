use clap::ArgMatches;

pub static UPDATE_CONTROL_VALUES: &[&str] = &["all", "none", "old", ""];

#[derive(Clone, Eq, PartialEq)]
pub enum UpdateMode {
    ReplaceAll,
    ReplaceNone,
    ReplaceIfOlder,
}

pub mod arguments {
    use clap::ArgAction;

    pub static OPT_UPDATE: &str = "update";
    pub static OPT_UPDATE_NO_ARG: &str = "u";

    pub fn update() -> clap::Arg {
        clap::Arg::new(OPT_UPDATE)
            .long("update")
            .help("move only when the SOURCE file is newer than the destination file or when the destination file is missing")
            .value_parser(["", "none", "all", "older"])
            .num_args(0..=1)
            .default_missing_value("older")
            .require_equals(true)
            .overrides_with("update")
            .action(clap::ArgAction::Set)
    }

    pub fn update_no_args() -> clap::Arg {
        clap::Arg::new(OPT_UPDATE_NO_ARG)
            .short('u')
            .help("like --update but does not accept an argument")
            .action(ArgAction::SetTrue)
    }
}

pub fn determine_update_mode(matches: &ArgMatches) -> UpdateMode {
    if matches.contains_id(arguments::OPT_UPDATE) {
        if let Some(mode) = matches.get_one::<String>(arguments::OPT_UPDATE) {
            match mode.as_str() {
                "all" | "" => UpdateMode::ReplaceAll,
                "none" => UpdateMode::ReplaceNone,
                "older" => UpdateMode::ReplaceIfOlder,
                _ => unreachable!("other args restricted by clap"),
            }
        } else {
            unreachable!("other args restricted by clap")
        }
    } else if matches.get_flag(arguments::OPT_UPDATE_NO_ARG) {
        UpdateMode::ReplaceIfOlder
    } else {
        UpdateMode::ReplaceAll
    }
}
