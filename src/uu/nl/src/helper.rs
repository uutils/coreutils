// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) conv

use std::ffi::OsString;
use std::num::IntErrorKind;

use uucore::translate;

use crate::{NumberingStyle, NumberingStyleError, options};

/// GNU reports a plain (no unit suffix) numeric option's own value with its
/// own wording, and immediately, fatally, exits on the first one it finds
/// invalid -- it does not keep parsing the rest of the command line the way
/// it does for the numbering-style options below.
///
/// GNU distinguishes two kinds of "doesn't fit": a value that doesn't fit
/// in `overflow_at`'s own underlying integer width at all (e.g. `-w`'s
/// value is stored in a plain C `int`, so anything outside `i32` overflows
/// it even though it parses as an `i64` just fine) is reported as "Value
/// too large for defined data type"; one that fits that width but falls
/// outside the option's own accepted `range` (e.g. `-w` additionally
/// requires a *positive* value) is reported as "Numerical result out of
/// range" instead. `kind` names which option this is, for the message.
fn parse_nl_number(
    value: &str,
    kind: &'static str,
    overflow_at: std::ops::RangeInclusive<i64>,
    range: std::ops::RangeInclusive<i64>,
) -> Result<i64, String> {
    match value.parse::<i64>() {
        Ok(n) if range.contains(&n) => Ok(n),
        Ok(n) if overflow_at.contains(&n) => Err(
            translate!("nl-error-number-out-of-range", "kind" => kind, "value" => value.to_owned()),
        ),
        Ok(_) => Err(
            translate!("nl-error-number-too-large", "kind" => kind, "value" => value.to_owned()),
        ),
        Err(e)
            if matches!(
                e.kind(),
                IntErrorKind::PosOverflow | IntErrorKind::NegOverflow
            ) =>
        {
            Err(
                translate!("nl-error-number-too-large", "kind" => kind, "value" => value.to_owned()),
            )
        }
        Err(_) => {
            Err(translate!("nl-error-invalid-number", "kind" => kind, "value" => value.to_owned()))
        }
    }
}

// parse_options loads the options into the settings, returning either the
// first immediately-fatal error found (matching GNU, which stops parsing the
// rest of the command line at that point), or the list of any non-fatal
// numbering-style/format errors collected along the way.
#[allow(clippy::cognitive_complexity)]
pub fn parse_options(
    settings: &mut crate::Settings,
    opts: &clap::ArgMatches,
) -> Result<Vec<String>, String> {
    let mut errs: Vec<String> = vec![];
    settings.renumber = opts.get_flag(options::NO_RENUMBER);

    if let Some(value) = opts.get_one::<String>(options::LINE_INCREMENT) {
        settings.line_increment = parse_nl_number(
            value,
            "line number increment",
            i64::MIN..=i64::MAX,
            i64::MIN..=i64::MAX,
        )?;
    }
    if let Some(value) = opts.get_one::<String>(options::JOIN_BLANK_LINES) {
        settings.join_blank_lines = parse_nl_number(
            value,
            "line number of blank lines",
            i64::MIN..=i64::MAX,
            0..=i64::MAX,
        )? as u64;
    }
    if let Some(value) = opts.get_one::<String>(options::STARTING_LINE_NUMBER) {
        settings.starting_line_number = parse_nl_number(
            value,
            "starting line number",
            i64::MIN..=i64::MAX,
            i64::MIN..=i64::MAX,
        )?;
    }
    if let Some(value) = opts.get_one::<String>(options::NUMBER_WIDTH) {
        settings.number_width = parse_nl_number(
            value,
            "line number field width",
            i64::from(i32::MIN)..=i64::from(i32::MAX),
            1..=i64::from(i32::MAX),
        )? as usize;
    }

    if let Some(mut delimiter) = opts
        .get_one::<OsString>(options::SECTION_DELIMITER)
        .cloned()
    {
        let is_single_char = delimiter
            .to_str()
            .map_or_else(|| delimiter.len() == 1, |s| s.chars().count() == 1);

        // A "single character" implies the second character of the delimiter is ':'.
        if is_single_char {
            delimiter.push(":");
        }

        settings.section_delimiter = delimiter;
    }

    if let Some(val) = opts.get_one::<OsString>(options::NUMBER_SEPARATOR) {
        settings.number_separator.clone_from(val);
    }

    // GNU reports these four in the order they were actually given on the
    // command line, not in any fixed order, so each is paired with its own
    // position for a final sort.
    let mut style_errs: Vec<(usize, String)> = vec![];

    if let Some(format) = opts.get_one::<String>(options::NUMBER_FORMAT) {
        match format.as_str() {
            "ln" | "rn" | "rz" => settings.number_format = format.clone().into(),
            _ => style_errs.push((
                opts.index_of(options::NUMBER_FORMAT).unwrap_or(0),
                translate!("nl-error-invalid-number-format", "value" => format.clone()),
            )),
        }
    }

    for (opt, kind, field) in [
        (
            options::HEADER_NUMBERING,
            "header",
            &mut settings.header_numbering,
        ),
        (
            options::BODY_NUMBERING,
            "body",
            &mut settings.body_numbering,
        ),
        (
            options::FOOTER_NUMBERING,
            "footer",
            &mut settings.footer_numbering,
        ),
    ] {
        if let Some(style) = opts.get_one::<String>(opt) {
            match NumberingStyle::try_from(style.as_str()) {
                Ok(numbering) => *field = numbering,
                Err(NumberingStyleError::InvalidRegex) => {
                    return Err(translate!("nl-error-invalid-regex"));
                }
                Err(NumberingStyleError::InvalidStyle) => style_errs.push((
                    opts.index_of(opt).unwrap_or(0),
                    translate!("nl-error-invalid-numbering-style", "kind" => kind, "value" => style.clone()),
                )),
            }
        }
    }

    style_errs.sort_by_key(|(index, _)| *index);
    errs.extend(style_errs.into_iter().map(|(_, message)| message));

    Ok(errs)
}
