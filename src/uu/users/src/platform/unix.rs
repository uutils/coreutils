// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) wtmp

use crate::uu_app;

use std::ffi::OsString;
use std::path::Path;

use uucore::error::UResult;
use uucore::utmpx::{self, Utmpx};

static ARG_FILES: &str = "files";

fn get_long_usage() -> String {
    format!(
        "Output who is currently logged in according to FILE.
If FILE is not specified, use {}.  /var/log/wtmp as FILE is common.",
        utmpx::DEFAULT_FILE
    )
}

pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(get_long_usage())
        .try_get_matches_from(args)?;

    let files: Vec<&Path> = matches
        .get_many::<OsString>(ARG_FILES)
        .map(|v| v.map(AsRef::as_ref).collect())
        .unwrap_or_default();

    let filename = if files.is_empty() {
        utmpx::DEFAULT_FILE.as_ref()
    } else {
        files[0]
    };

    let mut users = Utmpx::iter_all_records_from(filename)
        .filter(Utmpx::is_user_process)
        .map(|ut| ut.user())
        .collect::<Vec<_>>();

    if !users.is_empty() {
        users.sort();
        println!("{}", users.join(" "));
    }

    Ok(())
}
