// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) conv

use std::ffi::OsString;

use crate::{NumberingStyle, options};

// parse_options loads the options into the settings.
#[allow(clippy::cognitive_complexity)]
pub fn parse_options(settings: &mut crate::Settings, opts: &clap::ArgMatches) {
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
    if let Some(style) = opts.get_one::<NumberingStyle>(options::HEADER_NUMBERING) {
        settings.header_numbering = style.clone();
    }
    if let Some(style) = opts.get_one::<NumberingStyle>(options::BODY_NUMBERING) {
        settings.body_numbering = style.clone();
    }
    if let Some(style) = opts.get_one::<NumberingStyle>(options::FOOTER_NUMBERING) {
        settings.footer_numbering = style.clone();
    }
    if let Some(&num) = opts.get_one::<u64>(options::NUMBER_WIDTH) {
        settings.number_width = num as usize;
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
}
