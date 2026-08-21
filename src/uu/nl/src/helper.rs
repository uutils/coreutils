// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) conv

use std::ffi::OsString;

use crate::options;
use uucore::display::Quotable;
use uucore::translate;

// parse_options loads the options into the settings, returning an array of
// error messages.
#[allow(clippy::cognitive_complexity)]
pub fn parse_options(settings: &mut crate::Settings, opts: &clap::ArgMatches) -> Vec<String> {
    // This vector holds error messages encountered.
    let mut errs: Vec<String> = vec![];
    settings.renumber = opts.get_flag(options::NO_RENUMBER);

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
    settings.number_format = opts
        .get_one::<String>(options::NUMBER_FORMAT)
        .map(Into::into)
        .unwrap_or_default();
    match opts
        .get_one::<String>(options::HEADER_NUMBERING)
        .map(String::as_str)
        .map(TryInto::try_into)
    {
        None => {}
        Some(Ok(style)) => settings.header_numbering = style,
        Some(Err(message)) => errs.push(message),
    }
    match opts
        .get_one::<String>(options::BODY_NUMBERING)
        .map(String::as_str)
        .map(TryInto::try_into)
    {
        None => {}
        Some(Ok(style)) => settings.body_numbering = style,
        Some(Err(message)) => errs.push(message),
    }
    match opts
        .get_one::<String>(options::FOOTER_NUMBERING)
        .map(String::as_str)
        .map(TryInto::try_into)
    {
        None => {}
        Some(Ok(style)) => settings.footer_numbering = style,
        Some(Err(message)) => errs.push(message),
    }
    match opts.get_one::<usize>(options::NUMBER_WIDTH) {
        None => {}
        // GNU bounds the line-number field width to a C `int` ([1, INT_MAX]); a larger value
        // otherwise makes `number_width + 1` overflow `isize` and abort with a capacity overflow
        // when the blank prefix is built. Reject out-of-range widths up front, as GNU does.
        Some(num) if *num > 0 && i32::try_from(*num).is_ok() => settings.number_width = *num,
        Some(num) => {
            errs.push(translate!("nl-error-invalid-line-width", "value" => num.to_string().quote()))
        }
    }
    if let Some(num) = opts.get_one::<u64>(options::JOIN_BLANK_LINES) {
        settings.join_blank_lines = *num;
    }
    if let Some(num) = opts.get_one::<i64>(options::LINE_INCREMENT) {
        settings.line_increment = *num;
    }
    if let Some(num) = opts.get_one::<i64>(options::STARTING_LINE_NUMBER) {
        settings.starting_line_number = *num;
    }
    errs
}
