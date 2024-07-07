use clap::{crate_version, Command};
use uucore::{help_about, help_section};

pub const ABOUT: &str = help_about!("arch.md");
pub const SUMMARY: &str = help_section!("after help", "arch.md");

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(SUMMARY)
        .infer_long_args(true)
}
