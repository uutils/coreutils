extern crate getopts;
extern crate regex;

// parse_style parses a style string into a NumberingStyle.
fn parse_style(chars: &[char]) -> Result<::NumberingStyle, String> {
    match chars {
        ['a'] => { Ok(::NumberForAll) },
        ['t'] => { Ok(::NumberForNonEmpty) },
        ['n'] => { Ok(::NumberForNone) },
        ['p', rest..] => {
            match regex::Regex::new(String::from_chars(rest).as_slice()) {
                Ok(re) => Ok(::NumberForRegularExpression(re)),
                Err(_) => Err(String::from_str("Illegal regular expression")),
            }
        }
        _ => {
            Err(String::from_str("Illegal style encountered"))
        },
    }
}

// parse_options loads the options into the settings, returning an array of
// error messages.
pub fn parse_options(settings: &mut ::Settings, opts: &getopts::Matches) -> Vec<String> {
    // This vector holds error messages encountered.
    let mut errs: Vec<String> = vec![];
    settings.renumber = !opts.opt_present("p");
    match opts.opt_str("s") {
        None => {},
        Some(val) => { settings.number_separator = val; }
    }
    match opts.opt_str("n") {
        None => {},
        Some(val) => match val.as_slice() {
            "ln" => { settings.number_format = ::Left; },
            "rn" => { settings.number_format = ::Right; },
            "rz" => { settings.number_format = ::RightZero; },
            _ => { errs.push(String::from_str("Illegal value for -n")); },
        }
    }
    match opts.opt_str("b") {
        None => {},
        Some(val) => {
            let chars: Vec<char> = val.as_slice().chars().collect();
            match parse_style(chars.as_slice()) {
                Ok(s) => { settings.body_numbering = s; }
                Err(message) => { errs.push(message); }
            }
        }
    }
    match opts.opt_str("f") {
        None => {},
        Some(val) => {
            let chars: Vec<char> = val.as_slice().chars().collect();
            match parse_style(chars.as_slice()) {
                Ok(s) => { settings.footer_numbering = s; }
                Err(message) => { errs.push(message); }
            }
        }
    }
    match opts.opt_str("h") {
        None => {},
        Some(val) => {
            let chars: Vec<char> = val.as_slice().chars().collect();
            match parse_style(chars.as_slice()) {
                Ok(s) => { settings.header_numbering = s; }
                Err(message) => { errs.push(message); }
            }
        }
    }
    match opts.opt_str("i") {
        None => {}
        Some(val) => {
            let conv: Option<u64> = from_str(val.as_slice());
            match conv {
              None => {
                  errs.push(String::from_str("Illegal value for -i"));
              }
              Some(num) => { settings.line_increment = num }
            }
        }
    }
    match opts.opt_str("w") {
        None => {}
        Some(val) => {
            let conv: Option<uint> = from_str(val.as_slice());
            match conv {
              None => {
                  errs.push(String::from_str("Illegal value for -w"));
              }
              Some(num) => { settings.number_width = num }
            }
        }
    }
    match opts.opt_str("v") {
        None => {}
        Some(val) => {
            let conv: Option<u64> = from_str(val.as_slice());
            match conv {
              None => {
                  errs.push(String::from_str("Illegal value for -v"));
              }
              Some(num) => { settings.starting_line_number = num }
            }
        }
    }
    match opts.opt_str("l") {
        None => {}
        Some(val) => {
            let conv: Option<u64> = from_str(val.as_slice());
            match conv {
              None => {
                  errs.push(String::from_str("Illegal value for -l"));
              }
              Some(num) => { settings.join_blank_lines = num }
            }
        }
    }
    errs
}
