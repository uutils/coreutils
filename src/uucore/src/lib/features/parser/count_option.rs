// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Collection of the `-c`/`-n` values given on the command line.
//!
//! `head` and `tail` accept their count options more than once and have to
//! validate every value, including the ones that a later option overrides. The
//! commands declare both options with `ArgAction::Append`, so clap keeps all
//! of them; this module puts the values back into command-line order.

use clap::ArgMatches;

/// The count option a value belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CountOption {
    Bytes,
    Lines,
}

/// One `-c` or `-n` value as typed on the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountArgument {
    pub option: CountOption,
    pub value: String,
}

/// Collect every `-c`/`-n` value, including overridden ones, in the order
/// they appeared on the command line.
pub fn count_option_arguments(
    matches: &ArgMatches,
    bytes: &str,
    lines: &str,
) -> Vec<CountArgument> {
    let mut found: Vec<(usize, CountArgument)> = Vec::new();

    for (id, option) in [(bytes, CountOption::Bytes), (lines, CountOption::Lines)] {
        let (Some(values), Some(indices)) =
            (matches.get_many::<String>(id), matches.indices_of(id))
        else {
            continue;
        };
        found.extend(values.zip(indices).map(|(value, index)| {
            (
                index,
                CountArgument {
                    option,
                    value: value.clone(),
                },
            )
        }));
    }

    found.sort_by_key(|(index, _)| *index);
    found.into_iter().map(|(_, argument)| argument).collect()
}
