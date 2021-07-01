// spell-checker:ignore (ToDO) conv

use crate::options;

// parse_style parses a style string into a NumberingStyle.
fn parse_style(chars: &[char]) -> Result<crate::NumberingStyle, String> {
    if chars.len() == 1 && chars[0] == 'a' {
        Ok(crate::NumberingStyle::NumberForAll)
    } else if chars.len() == 1 && chars[0] == 't' {
        Ok(crate::NumberingStyle::NumberForNonEmpty)
    } else if chars.len() == 1 && chars[0] == 'n' {
        Ok(crate::NumberingStyle::NumberForNone)
    } else if chars.len() > 1 && chars[0] == 'p' {
        let s: String = chars[1..].iter().cloned().collect();
        match regex::Regex::new(&s) {
            Ok(re) => Ok(crate::NumberingStyle::NumberForRegularExpression(Box::new(
                re,
            ))),
            Err(_) => Err(String::from("Illegal regular expression")),
        }
    } else {
        Err(String::from("Illegal style encountered"))
    }
}

// parse_options loads the options into the settings, returning an array of
// error messages.
pub fn parse_options(settings: &mut crate::Settings, opts: &clap::ArgMatches) -> Vec<String> {
    // This vector holds error messages encountered.
    let mut errs: Vec<String> = vec![];
    settings.renumber = !opts.is_present(options::NO_RENUMBER);
    match opts.value_of(options::NUMBER_SEPARATOR) {
        None => {}
        Some(val) => {
            settings.number_separator = val.to_owned();
        }
    }
    match opts.value_of(options::NUMBER_FORMAT) {
        None => {}
        Some(val) => match val {
            "ln" => {
                settings.number_format = crate::NumberFormat::Left;
            }
            "rn" => {
                settings.number_format = crate::NumberFormat::Right;
            }
            "rz" => {
                settings.number_format = crate::NumberFormat::RightZero;
            }
            _ => {
                errs.push(String::from("Illegal value for -n"));
            }
        },
    }
    match opts.value_of(options::BODY_NUMBERING) {
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
    match opts.value_of(options::FOOTER_NUMBERING) {
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
    match opts.value_of(options::HEADER_NUMBERING) {
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
    match opts.value_of(options::LINE_INCREMENT) {
        None => {}
        Some(val) => {
            let conv: Option<u64> = val.parse().ok();
            match conv {
                None => {
                    errs.push(String::from("Illegal value for -i"));
                }
                Some(num) => settings.line_increment = num,
            }
        }
    }
    match opts.value_of(options::NUMBER_WIDTH) {
        None => {}
        Some(val) => {
            let conv: Option<usize> = val.parse().ok();
            match conv {
                None => {
                    errs.push(String::from("Illegal value for -w"));
                }
                Some(num) => settings.number_width = num,
            }
        }
    }
    match opts.value_of(options::STARTING_LINE_NUMBER) {
        None => {}
        Some(val) => {
            let conv: Option<u64> = val.parse().ok();
            match conv {
                None => {
                    errs.push(String::from("Illegal value for -v"));
                }
                Some(num) => settings.starting_line_number = num,
            }
        }
    }
    match opts.value_of(options::JOIN_BLANK_LINES) {
        None => {}
        Some(val) => {
            let conv: Option<u64> = val.parse().ok();
            match conv {
                None => {
                    errs.push(String::from("Illegal value for -l"));
                }
                Some(num) => settings.join_blank_lines = num,
            }
        }
    }
    errs
}
