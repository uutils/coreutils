// spell-checker:ignore (ToDO) conv

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
            Ok(re) => Ok(crate::NumberingStyle::NumberForRegularExpression(re)),
            Err(_) => Err(String::from("Illegal regular expression")),
        }
    } else {
        Err(String::from("Illegal style encountered"))
    }
}

// parse_options loads the options into the settings, returning an array of
// error messages.
pub fn parse_options(settings: &mut crate::Settings, opts: &getopts::Matches) -> Vec<String> {
    // This vector holds error messages encountered.
    let mut errs: Vec<String> = vec![];
    settings.renumber = !opts.opt_present("p");
    match opts.opt_str("s") {
        None => {}
        Some(val) => {
            settings.number_separator = val;
        }
    }
    match opts.opt_str("n") {
        None => {}
        Some(val) => match val.as_ref() {
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
    match opts.opt_str("b") {
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
    match opts.opt_str("f") {
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
    match opts.opt_str("h") {
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
    match opts.opt_str("i") {
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
    match opts.opt_str("w") {
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
    match opts.opt_str("v") {
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
    match opts.opt_str("l") {
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
