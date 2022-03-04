use super::options;
use clap::ArgMatches;

/// Abstraction for getopts
pub trait CommandLineOpts {
    /// returns all command line parameters which do not belong to an option.
    fn inputs(&self) -> Vec<&str>;
    /// tests if any of the specified options is present.
    fn opts_present(&self, _: &[&str]) -> bool;
}

/// Implementation for `getopts`
impl<'a> CommandLineOpts for ArgMatches {
    fn inputs(&self) -> Vec<&str> {
        self.values_of(options::FILENAME)
            .map(|values| values.collect())
            .unwrap_or_default()
    }

    fn opts_present(&self, opts: &[&str]) -> bool {
        opts.iter().any(|opt| self.is_present(opt))
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
    FileAndOffset((String, u64, Option<u64>)),
}

/// Interprets the command line inputs of od.
///
/// Returns either an unspecified number of filenames.
/// Or it will return a single filename, with an offset and optional label.
/// Offset and label are specified in bytes.
/// '-' is used as filename if stdin is meant. This is also returned if
/// there is no input, as stdin is the default input.
pub fn parse_inputs(matches: &dyn CommandLineOpts) -> Result<CommandLineInputs, String> {
    let mut input_strings = matches.inputs();

    if matches.opts_present(&["traditional"]) {
        return parse_inputs_traditional(&input_strings);
    }

    // test if command line contains: [file] <offset>
    // fall-through if no (valid) offset is found
    if input_strings.len() == 1 || input_strings.len() == 2 {
        // if any of the options -A, -j, -N, -t, -v or -w are present there is no offset
        if !matches.opts_present(&[
            options::ADDRESS_RADIX,
            options::READ_BYTES,
            options::SKIP_BYTES,
            options::FORMAT,
            options::OUTPUT_DUPLICATES,
            options::WIDTH,
        ]) {
            // test if the last input can be parsed as an offset.
            let offset = parse_offset_operand(input_strings[input_strings.len() - 1]);
            if let Ok(n) = offset {
                // if there is just 1 input (stdin), an offset must start with '+'
                if input_strings.len() == 1 && input_strings[0].starts_with('+') {
                    return Ok(CommandLineInputs::FileAndOffset(("-".to_string(), n, None)));
                }
                if input_strings.len() == 2 {
                    return Ok(CommandLineInputs::FileAndOffset((
                        input_strings[0].to_string(),
                        n,
                        None,
                    )));
                }
            }
        }
    }

    if input_strings.is_empty() {
        input_strings.push("-");
    }
    Ok(CommandLineInputs::FileNames(
        input_strings.iter().map(|&s| s.to_string()).collect(),
    ))
}

/// interprets inputs when --traditional is on the command line
///
/// normally returns CommandLineInputs::FileAndOffset, but if no offset is found,
/// it returns CommandLineInputs::FileNames (also to differentiate from the offset == 0)
pub fn parse_inputs_traditional(input_strings: &[&str]) -> Result<CommandLineInputs, String> {
    match input_strings.len() {
        0 => Ok(CommandLineInputs::FileNames(vec!["-".to_string()])),
        1 => {
            let offset0 = parse_offset_operand(input_strings[0]);
            Ok(match offset0 {
                Ok(n) => CommandLineInputs::FileAndOffset(("-".to_string(), n, None)),
                _ => CommandLineInputs::FileNames(
                    input_strings.iter().map(|&s| s.to_string()).collect(),
                ),
            })
        }
        2 => {
            let offset0 = parse_offset_operand(input_strings[0]);
            let offset1 = parse_offset_operand(input_strings[1]);
            match (offset0, offset1) {
                (Ok(n), Ok(m)) => Ok(CommandLineInputs::FileAndOffset((
                    "-".to_string(),
                    n,
                    Some(m),
                ))),
                (_, Ok(m)) => Ok(CommandLineInputs::FileAndOffset((
                    input_strings[0].to_string(),
                    m,
                    None,
                ))),
                _ => Err(format!("invalid offset: {}", input_strings[1])),
            }
        }
        3 => {
            let offset = parse_offset_operand(input_strings[1]);
            let label = parse_offset_operand(input_strings[2]);
            match (offset, label) {
                (Ok(n), Ok(m)) => Ok(CommandLineInputs::FileAndOffset((
                    input_strings[0].to_string(),
                    n,
                    Some(m),
                ))),
                (Err(_), _) => Err(format!("invalid offset: {}", input_strings[1])),
                (_, Err(_)) => Err(format!("invalid label: {}", input_strings[2])),
            }
        }
        _ => Err(format!(
            "too many inputs after --traditional: {}",
            input_strings[3]
        )),
    }
}

/// parses format used by offset and label on the command line
pub fn parse_offset_operand(s: &str) -> Result<u64, &'static str> {
    let mut start = 0;
    let mut len = s.len();
    let mut radix = 8;
    let mut multiply = 1;

    if s.starts_with('+') {
        start += 1;
    }

    if s[start..len].starts_with("0x") || s[start..len].starts_with("0X") {
        start += 2;
        radix = 16;
    } else {
        if s[start..len].ends_with('b') {
            len -= 1;
            multiply = 512;
        }
        if s[start..len].ends_with('.') {
            len -= 1;
            radix = 10;
        }
    }
    match u64::from_str_radix(&s[start..len], radix) {
        Ok(i) => Ok(i * multiply),
        Err(_) => Err("parse failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uu_app;

    #[test]
    fn test_parse_inputs_normal() {
        assert_eq!(
            CommandLineInputs::FileNames(vec!["-".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od"])).unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileNames(vec!["-".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "-"])).unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileNames(vec!["file1".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "file1"])).unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileNames(vec!["file1".to_string(), "file2".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "file1", "file2"])).unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileNames(vec![
                "-".to_string(),
                "file1".to_string(),
                "file2".to_string(),
            ]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "-", "file1", "file2"])).unwrap()
        );
    }

    #[test]
    fn test_parse_inputs_with_offset() {
        // offset is found without filename, so stdin will be used.
        assert_eq!(
            CommandLineInputs::FileAndOffset(("-".to_string(), 8, None)),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "+10"])).unwrap()
        );

        // offset must start with "+" if no input is specified.
        assert_eq!(
            CommandLineInputs::FileNames(vec!["10".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "10"])).unwrap()
        );

        // offset is not valid, so it is considered a filename.
        assert_eq!(
            CommandLineInputs::FileNames(vec!["+10a".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "+10a"])).unwrap()
        );

        // if -j is included in the command line, there cannot be an offset.
        assert_eq!(
            CommandLineInputs::FileNames(vec!["+10".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "-j10", "+10"])).unwrap()
        );

        // if -v is included in the command line, there cannot be an offset.
        assert_eq!(
            CommandLineInputs::FileNames(vec!["+10".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "-o", "-v", "+10"])).unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileAndOffset(("file1".to_string(), 8, None)),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "file1", "+10"])).unwrap()
        );

        // offset does not need to start with "+" if a filename is included.
        assert_eq!(
            CommandLineInputs::FileAndOffset(("file1".to_string(), 8, None)),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "file1", "10"])).unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileNames(vec!["file1".to_string(), "+10a".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "file1", "+10a"])).unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileNames(vec!["file1".to_string(), "+10".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "-j10", "file1", "+10"])).unwrap()
        );

        // offset must be last on the command line
        assert_eq!(
            CommandLineInputs::FileNames(vec!["+10".to_string(), "file1".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "+10", "file1"])).unwrap()
        );
    }

    #[test]
    fn test_parse_inputs_traditional() {
        // it should not return FileAndOffset to signal no offset was entered on the command line.
        assert_eq!(
            CommandLineInputs::FileNames(vec!["-".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "--traditional"])).unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileNames(vec!["file1".to_string()]),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "--traditional", "file1"])).unwrap()
        );

        // offset does not need to start with a +
        assert_eq!(
            CommandLineInputs::FileAndOffset(("-".to_string(), 8, None)),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "--traditional", "10"])).unwrap()
        );

        // valid offset and valid label
        assert_eq!(
            CommandLineInputs::FileAndOffset(("-".to_string(), 8, Some(8))),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "--traditional", "10", "10"]))
                .unwrap()
        );

        assert_eq!(
            CommandLineInputs::FileAndOffset(("file1".to_string(), 8, None)),
            parse_inputs(&uu_app().get_matches_from(vec!["od", "--traditional", "file1", "10"]))
                .unwrap()
        );

        // only one file is allowed, it must be the first
        parse_inputs(&uu_app().get_matches_from(vec!["od", "--traditional", "10", "file1"]))
            .unwrap_err();

        assert_eq!(
            CommandLineInputs::FileAndOffset(("file1".to_string(), 8, Some(8))),
            parse_inputs(&uu_app().get_matches_from(vec![
                "od",
                "--traditional",
                "file1",
                "10",
                "10"
            ]))
            .unwrap()
        );

        parse_inputs(&uu_app().get_matches_from(vec!["od", "--traditional", "10", "file1", "10"]))
            .unwrap_err();

        parse_inputs(&uu_app().get_matches_from(vec!["od", "--traditional", "10", "10", "file1"]))
            .unwrap_err();

        parse_inputs(&uu_app().get_matches_from(vec![
            "od",
            "--traditional",
            "10",
            "10",
            "10",
            "10",
        ]))
        .unwrap_err();
    }

    fn parse_offset_operand_str(s: &str) -> Result<u64, &'static str> {
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
