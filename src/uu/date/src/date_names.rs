// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore wednes

//! GNU-compatible date-name matching for input strings.

/// GNU documents these forms in its "Calendar date items" section:
/// <https://www.gnu.org/software/coreutils/manual/html_node/Calendar-date-items.html>
/// Full names, three-letter abbreviations, and the special `Sept` form are
/// accepted.
pub(super) fn is_month_name(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "january"
            | "jan"
            | "february"
            | "feb"
            | "march"
            | "mar"
            | "april"
            | "apr"
            | "may"
            | "june"
            | "jun"
            | "july"
            | "jul"
            | "august"
            | "aug"
            | "september"
            | "sep"
            | "sept"
            | "october"
            | "oct"
            | "november"
            | "nov"
            | "december"
            | "dec"
    )
}

/// GNU documents these forms in its "Day of week items" section:
/// <https://www.gnu.org/software/coreutils/manual/html_node/Day-of-week-items.html>
/// Full names, three-letter abbreviations with an optional period, and the
/// special `tues`, `wednes`, `thur`, and `thurs` forms are accepted. Single-
/// letter forms are intentionally excluded because they are ambiguous with
/// other syntax.
pub(super) fn is_weekday_name(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "monday"
            | "mon"
            | "mon."
            | "tuesday"
            | "tue"
            | "tue."
            | "tues"
            | "wednesday"
            | "wed"
            | "wed."
            | "wednes"
            | "thursday"
            | "thu"
            | "thu."
            | "thur"
            | "thurs"
            | "friday"
            | "fri"
            | "fri."
            | "saturday"
            | "sat"
            | "sat."
            | "sunday"
            | "sun"
            | "sun."
    )
}

pub(super) fn is_weekday_prefix(token: &str) -> bool {
    token.parse::<u32>().is_ok()
        || matches!(
            token.to_ascii_lowercase().as_str(),
            "first"
                | "second"
                | "third"
                | "fourth"
                | "fifth"
                | "sixth"
                | "seventh"
                | "eighth"
                | "ninth"
                | "tenth"
                | "eleventh"
                | "twelfth"
                | "last"
                | "next"
                | "this"
        )
}
