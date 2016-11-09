use getopts::Matches;

/// Abstraction for getopts
pub trait CommandLineOpts {
    /// returns all commandline parameters which do not belong to an option.
    fn inputs(&self) -> Vec<String>;
    /// tests if any of the specified options is present.
    fn opts_present(&self, &[&str]) -> bool;
}

/// Implementation for `getopts`
impl CommandLineOpts for Matches {
    fn inputs(&self) -> Vec<String> {
        self.free.clone()
    }
    fn opts_present(&self, opts: &[&str]) -> bool {
        self.opts_present(&opts.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
}

/// Contains the Input filename(s) with an optional offset.
///
/// `FileNames` is used for one or more file inputs ("-" = stdin)
/// `FileAndOffset` is used for a single file input, with an offset
/// and an optional label. Offset and label are specified in bytes.
/// `FileAndOffset` will be only used if an offset is specified,
/// but it might be 0.
#[derive(PartialEq, Debug)]
pub enum CommandLineInputs {
    FileNames(Vec<String>),
    FileAndOffset((String, usize, Option<usize>)),
}


/// Interprets the commandline inputs of od.
///
/// Returns either an unspecified number of filenames.
/// Or it will return a single filename, with an offset and optional label.
/// Offset and label are specified in bytes.
/// '-' is used as filename if stdin is meant. This is also returned if
/// there is no input, as stdin is the default input.
pub fn parse_inputs(matches: &CommandLineOpts) -> Result<CommandLineInputs, String> {
    let mut input_strings: Vec<String> = matches.inputs();

    if matches.opts_present(&["traditional"]) {
        return parse_inputs_traditional(input_strings);
    }

    // test if commandline contains: [file] <offset>
    // fall-through if no (valid) offset is found
    if input_strings.len() == 1 || input_strings.len() == 2 {
        // if any of the options -A, -j, -N, -t, -v or -w are present there is no offset
        if !matches.opts_present(&["A", "j", "N", "t", "v", "w"]) {
            // test if the last input can be parsed as an offset.
            let offset = parse_offset_operand(&input_strings[input_strings.len()-1]);
            match offset {
                Ok(n) => {
                    // if there is just 1 input (stdin), an offset must start with '+'
                    if input_strings.len() == 1 && input_strings[0].starts_with("+") {
                        return Ok(CommandLineInputs::FileAndOffset(("-".to_string(), n, None)));
                    }
                    if input_strings.len() == 2 {
                        return Ok(CommandLineInputs::FileAndOffset((input_strings[0].clone(), n, None)));
                    }
                }
                _ => {
                    // if it cannot be parsed, it is considered a filename
                }
            }
        }
    }

    if input_strings.len() == 0 {
        input_strings.push("-".to_string());
    }
    Ok(CommandLineInputs::FileNames(input_strings))
}

/// interprets inputs when --traditional is on the commandline
///
/// normally returns CommandLineInputs::FileAndOffset, but if no offset is found,
/// it returns CommandLineInputs::FileNames (also to differentiate from the offset == 0)
pub fn parse_inputs_traditional(input_strings: Vec<String>) -> Result<CommandLineInputs, String> {
    match input_strings.len() {
        0 => {
            Ok(CommandLineInputs::FileNames(vec!["-".to_string()]))
        }
        1 => {
            let offset0 = parse_offset_operand(&input_strings[0]);
            Ok(match offset0 {
                Ok(n) => CommandLineInputs::FileAndOffset(("-".to_string(), n, None)),
                _ => CommandLineInputs::FileNames(input_strings),
            })
        }
        2 => {
            let offset0 = parse_offset_operand(&input_strings[0]);
            let offset1 = parse_offset_operand(&input_strings[1]);
            match (offset0, offset1) {
                (Ok(n), Ok(m)) => Ok(CommandLineInputs::FileAndOffset(("-".to_string(), n, Some(m)))),
                (_, Ok(m)) => Ok(CommandLineInputs::FileAndOffset((input_strings[0].clone(), m, None))),
                _ => Err(format!("invalid offset: {}", input_strings[1])),
            }
        }
        3 => {
            let offset = parse_offset_operand(&input_strings[1]);
            let label = parse_offset_operand(&input_strings[2]);
            match (offset, label) {
                (Ok(n), Ok(m)) => Ok(CommandLineInputs::FileAndOffset((input_strings[0].clone(), n, Some(m)))),
                (Err(_), _) => Err(format!("invalid offset: {}", input_strings[1])),
                (_, Err(_)) => Err(format!("invalid label: {}", input_strings[2])),
            }
        }
        _ => {
            Err(format!("too many inputs after --traditional: {}", input_strings[3]))
        }
    }
}

/// parses format used by offset and label on the commandline
pub fn parse_offset_operand(s: &String) -> Result<usize, &'static str> {
    let mut start = 0;
    let mut len = s.len();
    let mut radix = 8;
    let mut multiply = 1;

    if s.starts_with("+") {
        start += 1;
    }

    if s[start..len].starts_with("0x") || s[start..len].starts_with("0X") {
        start += 2;
        radix = 16;
    } else {
        if s[start..len].ends_with("b") {
            len -= 1;
            multiply = 512;
        }
        if s[start..len].ends_with(".") {
            len -= 1;
            radix = 10;
        }
    }
    match usize::from_str_radix(&s[start..len], radix) {
        Ok(i) => Ok(i * multiply),
        Err(_) => Err("parse failed"),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// A mock for the commandline options type
    ///
    /// `inputs` are all commandline parameters which do not belong to an option.
    /// `option_names` are the names of the options on the commandline.
    struct MockOptions<'a> {
        inputs: Vec<String>,
        option_names: Vec<&'a str>,
    }

    impl<'a> MockOptions<'a> {
        fn new(inputs: Vec<&'a str>, option_names: Vec<&'a str>) -> MockOptions<'a> {
            MockOptions {
                inputs: inputs.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                option_names: option_names,
            }
        }
    }

    impl<'a> CommandLineOpts for MockOptions<'a> {
        fn inputs(&self) -> Vec<String> {
            self.inputs.clone()
        }
        fn opts_present(&self, opts: &[&str]) -> bool {
            for expected in opts.iter() {
                for actual in self.option_names.iter() {
                    if *expected == *actual {
                        return true;
                    }
                }
            }
            false
        }
    }

    #[test]
    fn test_parse_inputs_normal() {
        assert_eq!(CommandLineInputs::FileNames(vec!["-".to_string()]),
            parse_inputs(&MockOptions::new(
                vec![],
                vec![])).unwrap());

        assert_eq!(CommandLineInputs::FileNames(vec!["-".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["-"],
                vec![])).unwrap());

        assert_eq!(CommandLineInputs::FileNames(vec!["file1".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["file1"],
                vec![])).unwrap());

        assert_eq!(CommandLineInputs::FileNames(vec!["file1".to_string(), "file2".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["file1", "file2"],
                vec![])).unwrap());

        assert_eq!(CommandLineInputs::FileNames(vec!["-".to_string(), "file1".to_string(), "file2".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["-", "file1", "file2"],
                vec![])).unwrap());
    }

    #[test]
    fn test_parse_inputs_with_offset() {
        // offset is found without filename, so stdin will be used.
        assert_eq!(CommandLineInputs::FileAndOffset(("-".to_string(), 8, None)),
            parse_inputs(&MockOptions::new(
                vec!["+10"],
                vec![])).unwrap());

        // offset must start with "+" if no input is specified.
        assert_eq!(CommandLineInputs::FileNames(vec!["10".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["10"],
                vec![""])).unwrap());

        // offset is not valid, so it is considered a filename.
        assert_eq!(CommandLineInputs::FileNames(vec!["+10a".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["+10a"],
                vec![""])).unwrap());

        // if -j is included in the commandline, there cannot be an offset.
        assert_eq!(CommandLineInputs::FileNames(vec!["+10".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["+10"],
                vec!["j"])).unwrap());

        // if -v is included in the commandline, there cannot be an offset.
        assert_eq!(CommandLineInputs::FileNames(vec!["+10".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["+10"],
                vec!["o", "v"])).unwrap());

        assert_eq!(CommandLineInputs::FileAndOffset(("file1".to_string(), 8, None)),
            parse_inputs(&MockOptions::new(
                vec!["file1", "+10"],
                vec![])).unwrap());

        // offset does not need to start with "+" if a filename is included.
        assert_eq!(CommandLineInputs::FileAndOffset(("file1".to_string(), 8, None)),
            parse_inputs(&MockOptions::new(
                vec!["file1", "10"],
                vec![])).unwrap());

        assert_eq!(CommandLineInputs::FileNames(vec!["file1".to_string(), "+10a".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["file1", "+10a"],
                vec![""])).unwrap());

        assert_eq!(CommandLineInputs::FileNames(vec!["file1".to_string(), "+10".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["file1", "+10"],
                vec!["j"])).unwrap());

        // offset must be last on the commandline
        assert_eq!(CommandLineInputs::FileNames(vec!["+10".to_string(), "file1".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["+10", "file1"],
                vec![""])).unwrap());
    }

    #[test]
    fn test_parse_inputs_traditional() {
        // it should not return FileAndOffset to signal no offset was entered on the commandline.
        assert_eq!(CommandLineInputs::FileNames(vec!["-".to_string()]),
            parse_inputs(&MockOptions::new(
                vec![],
                vec!["traditional"])).unwrap());

        assert_eq!(CommandLineInputs::FileNames(vec!["file1".to_string()]),
            parse_inputs(&MockOptions::new(
                vec!["file1"],
                vec!["traditional"])).unwrap());

        // offset does not need to start with a +
        assert_eq!(CommandLineInputs::FileAndOffset(("-".to_string(), 8, None)),
            parse_inputs(&MockOptions::new(
                vec!["10"],
                vec!["traditional"])).unwrap());

        // valid offset and valid label
        assert_eq!(CommandLineInputs::FileAndOffset(("-".to_string(), 8, Some(8))),
            parse_inputs(&MockOptions::new(
                vec!["10", "10"],
                vec!["traditional"])).unwrap());

        assert_eq!(CommandLineInputs::FileAndOffset(("file1".to_string(), 8, None)),
            parse_inputs(&MockOptions::new(
                vec!["file1", "10"],
                vec!["traditional"])).unwrap());

        // only one file is allowed, it must be the first
        parse_inputs(&MockOptions::new(
                vec!["10", "file1"],
                vec!["traditional"])).unwrap_err();

        assert_eq!(CommandLineInputs::FileAndOffset(("file1".to_string(), 8, Some(8))),
            parse_inputs(&MockOptions::new(
                vec!["file1", "10", "10"],
                vec!["traditional"])).unwrap());

        parse_inputs(&MockOptions::new(
                vec!["10", "file1", "10"],
                vec!["traditional"])).unwrap_err();

        parse_inputs(&MockOptions::new(
                vec!["10", "10", "file1"],
                vec!["traditional"])).unwrap_err();

        parse_inputs(&MockOptions::new(
                vec!["10", "10", "10", "10"],
                vec!["traditional"])).unwrap_err();
    }

    fn parse_offset_operand_str(s: &str) -> Result<usize, &'static str> {
        parse_offset_operand(&String::from(s))
    }

    #[test]
    fn test_parse_offset_operand_invalid() {
        parse_offset_operand_str("").unwrap_err();
        parse_offset_operand_str("a").unwrap_err();
        parse_offset_operand_str("+").unwrap_err();
        parse_offset_operand_str("+b").unwrap_err();
        parse_offset_operand_str("0x1.").unwrap_err();
        parse_offset_operand_str("0x1.b").unwrap_err();
        parse_offset_operand_str("-").unwrap_err();
        parse_offset_operand_str("-1").unwrap_err();
        parse_offset_operand_str("1e10").unwrap_err();
    }

    #[test]
    fn test_parse_offset_operand() {
        assert_eq!(8, parse_offset_operand_str("10").unwrap()); // default octal
        assert_eq!(0, parse_offset_operand_str("0").unwrap());
        assert_eq!(8, parse_offset_operand_str("+10").unwrap()); // optional leading '+'
        assert_eq!(16, parse_offset_operand_str("0x10").unwrap()); // hex
        assert_eq!(16, parse_offset_operand_str("0X10").unwrap()); // hex
        assert_eq!(16, parse_offset_operand_str("+0X10").unwrap()); // hex
        assert_eq!(10, parse_offset_operand_str("10.").unwrap()); // decimal
        assert_eq!(10, parse_offset_operand_str("+10.").unwrap()); // decimal
        assert_eq!(4096, parse_offset_operand_str("10b").unwrap()); // b suffix = *512
        assert_eq!(4096, parse_offset_operand_str("+10b").unwrap()); // b suffix = *512
        assert_eq!(5120, parse_offset_operand_str("10.b").unwrap()); // b suffix = *512
        assert_eq!(5120, parse_offset_operand_str("+10.b").unwrap()); // b suffix = *512
        assert_eq!(267, parse_offset_operand_str("0x10b").unwrap()); // hex
    }
}
