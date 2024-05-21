// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Implement GNU-style update functionality.
//!
//! - pre-defined [`clap`-Arguments][1] for inclusion in utilities that
//!   implement updates
//! - determination of the [update mode][2]
//!
//! Update-functionality is implemented by the following utilities:
//!
//! - `cp`
//! - `mv`
//!
//!
//! [1]: arguments
//! [2]: `determine_update_mode()`
//!
//!
//! # Usage example
//!
//! ```
//! #[macro_use]
//! extern crate uucore;
//!
//! use clap::{Command, Arg, ArgMatches};
//! use uucore::update_control::{self, UpdateMode};
//!
//! fn main() {
//!     let matches = Command::new("command")
//!         .arg(update_control::arguments::update())
//!         .arg(update_control::arguments::update_no_args())
//!         .get_matches_from(vec![
//!             "command", "--update=older"
//!         ]);
//!
//!     let update_mode = update_control::determine_update_mode(&matches);
//!
//!     // handle cases
//!     if update_mode == UpdateMode::ReplaceIfOlder {
//!         // do
//!     } else {
//!         unreachable!()
//!     }
//! }
//! ```
use clap::ArgMatches;

// Available update mode
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateMode {
    // --update=`all`, ``
    ReplaceAll,
    // --update=`none`
    ReplaceNone,
    // --update=`older`
    // -u
    ReplaceIfOlder,
}

pub mod arguments {
    use crate::shortcut_value_parser::ShortcutValueParser;
    use clap::ArgAction;

    pub static OPT_UPDATE: &str = "update";
    pub static OPT_UPDATE_NO_ARG: &str = "u";

    // `--update` argument, defaults to `older` if no values are provided
    pub fn update() -> clap::Arg {
        clap::Arg::new(OPT_UPDATE)
            .long("update")
            .help("move only when the SOURCE file is newer than the destination file or when the destination file is missing")
            .value_parser(ShortcutValueParser::new(["none", "all", "older"]))
            .num_args(0..=1)
            .default_missing_value("older")
            .require_equals(true)
            .overrides_with("update")
            .action(clap::ArgAction::Set)
    }

    // `-u` argument
    pub fn update_no_args() -> clap::Arg {
        clap::Arg::new(OPT_UPDATE_NO_ARG)
            .short('u')
            .help("like --update but does not accept an argument")
            .action(ArgAction::SetTrue)
    }
}

/// Determine the "mode" for the update operation to perform, if any.
///
/// Parses the backup options and converts them to an instance of
/// `UpdateMode` for further processing.
///
/// Takes [`clap::ArgMatches`] as argument which **must** contain the options
/// from [`arguments::update()`] or [`arguments::update_no_args()`]. Otherwise
/// the `ReplaceAll` mode is returned unconditionally.
///
/// # Examples
///
/// Here's how one would integrate the update mode determination into an
/// application.
///
/// ```
/// #[macro_use]
/// extern crate uucore;
/// use uucore::update_control::{self, UpdateMode};
/// use clap::{Command, Arg, ArgMatches};
///
/// fn main() {
///     let matches = Command::new("command")
///         .arg(update_control::arguments::update())
///         .arg(update_control::arguments::update_no_args())
///         .get_matches_from(vec![
///             "command", "--update=all"
///         ]);
///
///     let update_mode = update_control::determine_update_mode(&matches);
///     assert_eq!(update_mode, UpdateMode::ReplaceAll)
/// }
pub fn determine_update_mode(matches: &ArgMatches) -> UpdateMode {
    if let Some(mode) = matches.get_one::<String>(arguments::OPT_UPDATE) {
        match mode.as_str() {
            "all" => UpdateMode::ReplaceAll,
            "none" => UpdateMode::ReplaceNone,
            "older" => UpdateMode::ReplaceIfOlder,
            _ => unreachable!("other args restricted by clap"),
        }
    } else if matches.get_flag(arguments::OPT_UPDATE_NO_ARG) {
        // short form of this option is equivalent to using --update=older
        UpdateMode::ReplaceIfOlder
    } else {
        // no option was present
        UpdateMode::ReplaceAll
    }
}
