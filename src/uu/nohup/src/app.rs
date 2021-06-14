use clap::{crate_version, App, AppSettings, Arg};

const ABOUT: &str = "Run COMMAND ignoring hangup signals.";
const LONG_HELP: &str = "
If standard input is terminal, it'll be replaced with /dev/null.
If standard output is terminal, it'll be appended to nohup.out instead,
or $HOME/nohup.out, if nohup.out open failed.
If standard error is terminal, it'll be redirected to stdout.
";

pub mod options {
    pub const CMD: &str = "cmd";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::CMD)
                .hidden(true)
                .required(true)
                .multiple(true),
        )
        .setting(AppSettings::TrailingVarArg)
}
