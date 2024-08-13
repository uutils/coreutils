// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) conv

use crate::options;

// parse_options loads the options into the settings, returning an array of
// error messages.
#[allow(clippy::cognitive_complexity)]
pub fn parse_options(settings: &mut crate::Settings, opts: &clap::ArgMatches) -> Vec<String> {
    // This vector holds error messages encountered.
    let mut errs: Vec<String> = vec![];
    settings.renumber = opts.get_flag(options::NO_RENUMBER);
    if let Some(delimiter) = opts.get_one::<String>(options::SECTION_DELIMITER) {
        // check whether the delimiter is a single ASCII char (1 byte)
        // because GNU nl doesn't add a ':' to single non-ASCII chars
        settings.section_delimiter = if delimiter.len() == 1 {
            format!("{delimiter}:")
        } else {
            delimiter.clone()
        };
    }
    if let Some(val) = opts.get_one::<String>(options::NUMBER_SEPARATOR) {
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
        Some(num) if *num > 0 => settings.number_width = *num,
        Some(_) => errs.push(String::from(
            "Invalid line number field width: ‘0’: Numerical result out of range",
        )),
    }
    match opts.get_one::<u64>(options::JOIN_BLANK_LINES) {
        None => {}
        Some(num) if *num > 0 => settings.join_blank_lines = *num,
        Some(_) => errs.push(String::from(
            "Invalid line number of blank lines: ‘0’: Numerical result out of range",
        )),
    }
    if let Some(num) = opts.get_one::<i64>(options::LINE_INCREMENT) {
        settings.line_increment = *num;
    }
    if let Some(num) = opts.get_one::<i64>(options::STARTING_LINE_NUMBER) {
        settings.starting_line_number = *num;
    }
    errs
}
