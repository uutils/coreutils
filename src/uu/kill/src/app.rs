// spell-checker:ignore (ToDO) pids

use clap::{crate_version, App, Arg};

pub mod options {
    pub const PIDS_OR_SIGNALS: &str = "pids_or_signals";
    pub const LIST: &str = "list";
    pub const TABLE: &str = "table";
    pub const TABLE_OLD: &str = "table_old";
    pub const SIGNAL: &str = "signal";
}

pub const ABOUT: &str = "Send signal to processes or list information about signals.";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::LIST)
                .short("l")
                .long(options::LIST)
                .help("Lists signals")
                .conflicts_with(options::TABLE)
                .conflicts_with(options::TABLE_OLD),
        )
        .arg(
            Arg::with_name(options::TABLE)
                .short("t")
                .long(options::TABLE)
                .help("Lists table of signals"),
        )
        .arg(Arg::with_name(options::TABLE_OLD).short("L").hidden(true))
        .arg(
            Arg::with_name(options::SIGNAL)
                .short("s")
                .long(options::SIGNAL)
                .help("Sends given signal")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::PIDS_OR_SIGNALS)
                .hidden(true)
                .multiple(true),
        )
}
