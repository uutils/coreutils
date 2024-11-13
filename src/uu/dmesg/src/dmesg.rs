use clap::{crate_version, Command};
use uucore::error::UResult;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name()).version(crate_version!())
}
