use formatteriteminfo::FormatterItemInfo;
use prn_int::*;
use prn_char::*;
use prn_float::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ParsedFormatterItemInfo {
    pub formatter_item_info:  FormatterItemInfo,
    pub add_ascii_dump: bool,
}

impl ParsedFormatterItemInfo {
    pub fn new(formatter_item_info: FormatterItemInfo, add_ascii_dump: bool) -> ParsedFormatterItemInfo {
        ParsedFormatterItemInfo {
            formatter_item_info: formatter_item_info,
            add_ascii_dump: add_ascii_dump,
        }
    }
}

fn od_argument_traditional_format(ch: char) -> Option<FormatterItemInfo> {
    match ch {
        'a' => Some(FORMAT_ITEM_A),
        'B' => Some(FORMAT_ITEM_OCT16),
        'b' => Some(FORMAT_ITEM_OCT8),
        'c' => Some(FORMAT_ITEM_C),
        'D' => Some(FORMAT_ITEM_DEC32U),
        'd' => Some(FORMAT_ITEM_DEC16U),
        'e' => Some(FORMAT_ITEM_F64),
        'F' => Some(FORMAT_ITEM_F64),
        'f' => Some(FORMAT_ITEM_F32),
        'H' => Some(FORMAT_ITEM_HEX32),
        'h' => Some(FORMAT_ITEM_HEX16),
        'i' => Some(FORMAT_ITEM_DEC32S),
        'I' => Some(FORMAT_ITEM_DEC64S),
        'L' => Some(FORMAT_ITEM_DEC64S),
        'l' => Some(FORMAT_ITEM_DEC64S),
        'O' => Some(FORMAT_ITEM_OCT32),
        'o' => Some(FORMAT_ITEM_OCT16),
        's' => Some(FORMAT_ITEM_DEC16S),
        'X' => Some(FORMAT_ITEM_HEX32),
        'x' => Some(FORMAT_ITEM_HEX16),
        _ => None,
    }
}

fn od_format_type(type_char: FormatType, byte_size: u8) -> Option<FormatterItemInfo> {
    match (type_char, byte_size) {
        (FormatType::Ascii, _) => Some(FORMAT_ITEM_A),
        (FormatType::Char, _) => Some(FORMAT_ITEM_C),

        (FormatType::DecimalInt, 1) => Some(FORMAT_ITEM_DEC8S),
        (FormatType::DecimalInt, 2) => Some(FORMAT_ITEM_DEC16S),
        (FormatType::DecimalInt, 0) |
        (FormatType::DecimalInt, 4) => Some(FORMAT_ITEM_DEC32S),
        (FormatType::DecimalInt, 8) => Some(FORMAT_ITEM_DEC64S),

        (FormatType::OctalInt, 1) => Some(FORMAT_ITEM_OCT8),
        (FormatType::OctalInt, 2) => Some(FORMAT_ITEM_OCT16),
        (FormatType::OctalInt, 0) |
        (FormatType::OctalInt, 4) => Some(FORMAT_ITEM_OCT32),
        (FormatType::OctalInt, 8) => Some(FORMAT_ITEM_OCT64),

        (FormatType::UnsignedInt, 1) => Some(FORMAT_ITEM_DEC8U),
        (FormatType::UnsignedInt, 2) => Some(FORMAT_ITEM_DEC16U),
        (FormatType::UnsignedInt, 0) |
        (FormatType::UnsignedInt, 4) => Some(FORMAT_ITEM_DEC32U),
        (FormatType::UnsignedInt, 8) => Some(FORMAT_ITEM_DEC64U),

        (FormatType::HexadecimalInt, 1) => Some(FORMAT_ITEM_HEX8),
        (FormatType::HexadecimalInt, 2) => Some(FORMAT_ITEM_HEX16),
        (FormatType::HexadecimalInt, 0) |
        (FormatType::HexadecimalInt, 4) => Some(FORMAT_ITEM_HEX32),
        (FormatType::HexadecimalInt, 8) => Some(FORMAT_ITEM_HEX64),

        (FormatType::Float, 2) => Some(FORMAT_ITEM_F16),
        (FormatType::Float, 0) |
        (FormatType::Float, 4) => Some(FORMAT_ITEM_F32),
        (FormatType::Float, 8) => Some(FORMAT_ITEM_F64),

        _ => None,
    }
}

fn od_argument_with_option(ch:char) -> bool {
    match ch {
        'A' | 'j' | 'N' | 'S' | 'w' => true,
        _ => false,
    }
}


/// Parses format flags from commandline
///
/// getopts, docopt, clap don't seem suitable to parse the commandline
/// arguments used for formats. In particular arguments can appear
/// multiple times and the order they appear in, is significant.
///
/// arguments like -f, -o, -x can appear separate or combined: -fox
/// it can also be mixed with non format related flags like -v: -fvox
/// arguments with parameters like -w16 can only appear at the end: -fvoxw16
/// parameters of -t/--format specify 1 or more formats.
/// if -- appears on the commandline, parsing should stop.
pub fn parse_format_flags(args: &Vec<String>) -> Result<Vec<ParsedFormatterItemInfo>, String> {
    let mut formats = Vec::new();

    // args[0] is the name of the binary
    let mut arg_iter = args.iter().skip(1);
    let mut expect_type_string = false;

    while let Some(arg) = arg_iter.next() {
        if expect_type_string {
            match parse_type_string(arg) {
                Ok(v) => formats.extend(v.into_iter()),
                Err(e) => return Err(e),
            }
            expect_type_string = false;
        } else if arg.starts_with("--") {
            if arg.len() == 2 {
                break;
            }
            if arg.starts_with("--format=") {
                let params: String = arg.chars().skip_while(|c| *c != '=').skip(1).collect();
                match parse_type_string(&params) {
                    Ok(v) => formats.extend(v.into_iter()),
                    Err(e) => return Err(e),
                }
            }
            if arg == "--format" {
                expect_type_string = true;
            }
        } else if arg.starts_with("-") {
            let mut flags = arg.chars().skip(1);
            let mut format_spec = String::new();
            while let Some(c) = flags.next() {
                if expect_type_string {
                    format_spec.push(c);
                } else if od_argument_with_option(c) {
                    break;
                } else if c == 't' {
                    expect_type_string = true;
                } else {
                    // not every option is a format
                    if let Some(r) = od_argument_traditional_format(c) {
                        formats.push(ParsedFormatterItemInfo::new(r, false))
                    }
                }
            }
            if !format_spec.is_empty() {
                match parse_type_string(&format_spec) {
                    Ok(v) => formats.extend(v.into_iter()),
                    Err(e) => return Err(e),
                }
                expect_type_string = false;
            }
        }
    }
    if expect_type_string {
        return Err(format!("missing format specification after '--format' / '-t'"));
    }

    if formats.is_empty() {
        formats.push(ParsedFormatterItemInfo::new(FORMAT_ITEM_OCT16, false)); // 2 byte octal is the default
    }

    Ok(formats)
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum FormatType {
    Ascii,
    Char,
    DecimalInt,
    OctalInt,
    UnsignedInt,
    HexadecimalInt,
    Float,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum FormatTypeCategory {
    Char,
    Integer,
    Float,
}

fn format_type(ch: char) -> Option<FormatType> {
    match ch {
        'a' => Some(FormatType::Ascii),
        'c' => Some(FormatType::Char),
        'd' => Some(FormatType::DecimalInt),
        'o' => Some(FormatType::OctalInt),
        'u' => Some(FormatType::UnsignedInt),
        'x' => Some(FormatType::HexadecimalInt),
        'f' => Some(FormatType::Float),
        _ => None,
    }
}


fn format_type_category(t: FormatType) -> FormatTypeCategory {
    match t {
        FormatType::Ascii | FormatType::Char
            => FormatTypeCategory::Char,
        FormatType::DecimalInt | FormatType::OctalInt | FormatType::UnsignedInt | FormatType::HexadecimalInt
            => FormatTypeCategory::Integer,
        FormatType::Float
            => FormatTypeCategory::Float,
    }
}

fn is_format_size_char(ch: Option<char>, format_type: FormatTypeCategory, byte_size: &mut u8) -> bool {
    match (format_type, ch) {
        (FormatTypeCategory::Integer, Some('C')) => {
            *byte_size = 1;
            true
        },
        (FormatTypeCategory::Integer, Some('S')) => {
            *byte_size = 2;
            true
        },
        (FormatTypeCategory::Integer, Some('I')) => {
            *byte_size = 4;
            true
        },
        (FormatTypeCategory::Integer, Some('L')) => {
            *byte_size = 8;
            true
        },

        (FormatTypeCategory::Float, Some('F')) => {
            *byte_size = 4;
            true
        },
        (FormatTypeCategory::Float, Some('D')) => {
            *byte_size = 8;
            true
        },
        // FormatTypeCategory::Float, 'L' => *byte_size = 16, // TODO support f128

        _ => false,
    }
}

fn is_format_size_decimal(ch: Option<char>, format_type: FormatTypeCategory, decimal_size: &mut String) -> bool {
    if format_type == FormatTypeCategory::Char { return false; }
    match ch {
        Some(d) if d.is_digit(10) => {
            decimal_size.push(d);
            return true;
        }
        _ => false,
    }
}

fn is_format_dump_char(ch: Option<char>, show_ascii_dump: &mut bool) -> bool {
    match ch {
        Some('z') => {
            *show_ascii_dump = true;
            return true;
        }
        _ => false,
    }
}

fn parse_type_string(params: &String) -> Result<Vec<ParsedFormatterItemInfo>, String> {
    let mut formats = Vec::new();

    let mut chars = params.chars();
    let mut ch = chars.next();

    while ch.is_some() {
        let type_char = ch.unwrap();
        let type_char = match format_type(type_char) {
            Some(t) => t,
            None => {
                return Err(format!("unexpected char '{}' in format specification '{}'", type_char, params));
            }
        };

        let type_cat = format_type_category(type_char);

        ch = chars.next();

        let mut byte_size = 0u8;
        let mut show_ascii_dump = false;
        if is_format_size_char(ch, type_cat, &mut byte_size) {
            ch = chars.next();
        } else {
            let mut decimal_size = String::new();
            while is_format_size_decimal(ch, type_cat, &mut decimal_size) {
                ch = chars.next();
            }
            if !decimal_size.is_empty() {
                byte_size = match decimal_size.parse() {
                    Err(_) => return Err(format!("invalid number '{}' in format specification '{}'", decimal_size, params)),
                    Ok(n) => n,
                }
            }
        }
        if is_format_dump_char(ch, &mut show_ascii_dump) {
            ch = chars.next();
        }

        match od_format_type(type_char, byte_size) {
            Some(ft) => formats.push(ParsedFormatterItemInfo::new(ft, show_ascii_dump)),
            None => return Err(format!("invalid size '{}' in format specification '{}'", byte_size, params)),
        }
    }

    Ok(formats)
}

#[cfg(test)]
pub fn parse_format_flags_str(args_str: &Vec<&'static str>) -> Result<Vec<FormatterItemInfo>, String> {
    let args = args_str.iter().map(|s| s.to_string()).collect();
    match parse_format_flags(&args) {
        Err(e) => Err(e),
        Ok(v) => {
            // tests using this function asume add_ascii_dump is not set
            Ok(v.into_iter()
                .inspect(|f| assert!(!f.add_ascii_dump))
                .map(|f| f.formatter_item_info)
                .collect())
        },
    }
}


#[test]
fn test_no_options() {
    assert_eq!(parse_format_flags_str(
        &vec!("od")).unwrap(),
        vec!(FORMAT_ITEM_OCT16));
}

#[test]
fn test_one_option() {
    assert_eq!(parse_format_flags_str(
        &vec!("od", "-F")).unwrap(),
        vec!(FORMAT_ITEM_F64));
}

#[test]
fn test_two_separate_options() {
    assert_eq!(parse_format_flags_str(
        &vec!("od", "-F", "-x")).unwrap(),
        vec!(FORMAT_ITEM_F64, FORMAT_ITEM_HEX16));
}

#[test]
fn test_two_combined_options() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-Fx")).unwrap(),
       vec!(FORMAT_ITEM_F64, FORMAT_ITEM_HEX16));
}

#[test]
fn test_ignore_non_format_parameters() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-d", "-Ax")).unwrap(),
       vec!(FORMAT_ITEM_DEC16U));
}

#[test]
fn test_ignore_separate_parameters() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-I", "-A", "x")).unwrap(),
       vec!(FORMAT_ITEM_DEC64S));
}

#[test]
fn test_ignore_trailing_vals() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-D", "--", "-x")).unwrap(),
       vec!(FORMAT_ITEM_DEC32U));
}

#[test]
fn test_invalid_long_format() {
    parse_format_flags_str(&vec!("od", "--format=X")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=xX")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=aC")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=fI")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=xD")).unwrap_err();

    parse_format_flags_str(&vec!("od", "--format=xC1")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=x1C")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=xz1")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=xzC")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=xzz")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=xCC")).unwrap_err();

    parse_format_flags_str(&vec!("od", "--format=c1")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=x256")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=d5")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format=f1")).unwrap_err();
}

#[test]
fn test_long_format_a() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=a")).unwrap(),
       vec!(FORMAT_ITEM_A));
}

#[test]
fn test_long_format_cz() {
   assert_eq!(parse_format_flags(
       &vec!("od".to_string(), "--format=cz".to_string())).unwrap(),
       vec!(ParsedFormatterItemInfo::new(FORMAT_ITEM_C, true)));
}

#[test]
fn test_long_format_d() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=d8")).unwrap(),
       vec!(FORMAT_ITEM_DEC64S));
}

#[test]
fn test_long_format_d_default() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=d")).unwrap(),
       vec!(FORMAT_ITEM_DEC32S));
}

#[test]
fn test_long_format_o_default() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=o")).unwrap(),
       vec!(FORMAT_ITEM_OCT32));
}

#[test]
fn test_long_format_u_default() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=u")).unwrap(),
       vec!(FORMAT_ITEM_DEC32U));
}

#[test]
fn test_long_format_x_default() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=x")).unwrap(),
       vec!(FORMAT_ITEM_HEX32));
}

#[test]
fn test_long_format_f_default() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=f")).unwrap(),
       vec!(FORMAT_ITEM_F32));
}

#[test]
fn test_long_format_next_arg() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format", "f8")).unwrap(),
       vec!(FORMAT_ITEM_F64));
}

#[test]
fn test_short_format_next_arg() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-t", "x8")).unwrap(),
       vec!(FORMAT_ITEM_HEX64));
}

#[test]
fn test_short_format_combined_arg() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-tu8")).unwrap(),
       vec!(FORMAT_ITEM_DEC64U));
}

#[test]
fn test_format_next_arg_invalid() {
    parse_format_flags_str(&vec!("od", "--format", "-v")).unwrap_err();
    parse_format_flags_str(&vec!("od", "--format")).unwrap_err();
    parse_format_flags_str(&vec!("od", "-t", "-v")).unwrap_err();
    parse_format_flags_str(&vec!("od", "-t")).unwrap_err();
}

#[test]
fn test_mixed_formats() {
   assert_eq!(parse_format_flags(
       &vec!(
           "od".to_string(),
           "--skip-bytes=2".to_string(),
           "-vItu1z".to_string(),
           "-N".to_string(),
           "1000".to_string(),
           "-xt".to_string(),
           "acdx1".to_string(),
           "--format=u2c".to_string(),
           "--format".to_string(),
           "f".to_string(),
           "-xAx".to_string(),
           "--".to_string(),
           "-h".to_string(),
           "--format=f8".to_string())).unwrap(),
       vec!(
           ParsedFormatterItemInfo::new(FORMAT_ITEM_DEC64S, false),  // I
           ParsedFormatterItemInfo::new(FORMAT_ITEM_DEC8U, true),    // tu1z
           ParsedFormatterItemInfo::new(FORMAT_ITEM_HEX16, false),   // x
           ParsedFormatterItemInfo::new(FORMAT_ITEM_A, false),       // ta
           ParsedFormatterItemInfo::new(FORMAT_ITEM_C, false),       // tc
           ParsedFormatterItemInfo::new(FORMAT_ITEM_DEC32S, false),  // td
           ParsedFormatterItemInfo::new(FORMAT_ITEM_HEX8, false),    // tx1
           ParsedFormatterItemInfo::new(FORMAT_ITEM_DEC16U, false),  // tu2
           ParsedFormatterItemInfo::new(FORMAT_ITEM_C, false),       // tc
           ParsedFormatterItemInfo::new(FORMAT_ITEM_F32, false),     // tf
           ParsedFormatterItemInfo::new(FORMAT_ITEM_HEX16, false),   // x
       ));
}
