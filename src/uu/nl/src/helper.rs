// spell-checker:ignore (ToDO) conv

use crate::options;

// parse_style parses a style string into a NumberingStyle.
fn parse_style(chars: &[char]) -> Result<crate::NumberingStyle, String> {
    if chars.len() == 1 && chars[0] == 'a' {
        Ok(crate::NumberingStyle::All)
    } else if chars.len() == 1 && chars[0] == 't' {
        Ok(crate::NumberingStyle::NonEmpty)
    } else if chars.len() == 1 && chars[0] == 'n' {
        Ok(crate::NumberingStyle::None)
    } else if chars.len() > 1 && chars[0] == 'p' {
        let s: String = chars[1..].iter().cloned().collect();
        match regex::Regex::new(&s) {
            Ok(re) => Ok(crate::NumberingStyle::Regex(Box::new(re))),
            Err(_) => Err(String::from("Illegal regular expression")),
        }
    } else {
        Err(String::from("Illegal style encountered"))
    }
}

// parse_options loads the options into the settings, returning an array of
// error messages.
#[allow(clippy::cognitive_complexity)]
pub fn parse_options(settings: &mut crate::Settings, opts: &clap::ArgMatches) -> Vec<String> {
    // This vector holds error messages encountered.
    let mut errs: Vec<String> = vec![];
    settings.renumber = opts.get_flag(options::NO_RENUMBER);
    if let Some(val) = opts.get_one::<String>(options::NUMBER_SEPARATOR) {
        settings.number_separator = val.to_owned();
    }
    settings.number_format = opts
        .get_one::<String>(options::NUMBER_FORMAT)
        .map(Into::into)
        .unwrap_or_default();
    match opts.get_one::<String>(options::BODY_NUMBERING) {
        None => {}
        Some(val) => {
            let chars: Vec<char> = val.chars().collect();
            match parse_style(&chars) {
                Ok(s) => {
                    settings.body_numbering = s;
                }
                Err(message) => {
                    errs.push(message);
                }
            }
        }
    }
    match opts.get_one::<String>(options::FOOTER_NUMBERING) {
        None => {}
        Some(val) => {
            let chars: Vec<char> = val.chars().collect();
            match parse_style(&chars) {
                Ok(s) => {
                    settings.footer_numbering = s;
                }
                Err(message) => {
                    errs.push(message);
                }
            }
        }
    }
    match opts.get_one::<String>(options::HEADER_NUMBERING) {
        None => {}
        Some(val) => {
            let chars: Vec<char> = val.chars().collect();
            match parse_style(&chars) {
                Ok(s) => {
                    settings.header_numbering = s;
                }
                Err(message) => {
                    errs.push(message);
                }
            }
        }
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
