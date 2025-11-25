// Specific implementation for OpenBSD: tool unsupported (utmpx not supported)

use crate::uu_app;
use uucore::error::UResult;
use uucore::translate;

pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let _matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    println!("{}", translate!("pinky-unsupported-openbsd"));
    Ok(())
}
