use std::collections::HashSet;
use formatteriteminfo::FormatterItemInfo;
use prn_int::*;
use prn_char::*;
use prn_float::*;

//This is available in some versions of std, but not all that we target.
macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
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
pub fn parse_format_flags(args: &Vec<String>) -> Result<Vec<FormatterItemInfo>, String> {

    let known_formats = hashmap![
    'a' => FORMAT_ITEM_A,
    'B' => FORMAT_ITEM_OCT16,
    'b' => FORMAT_ITEM_OCT8,
    'c' => FORMAT_ITEM_C,
    'D' => FORMAT_ITEM_DEC32U,
    'd' => FORMAT_ITEM_DEC16U,
    'e' => FORMAT_ITEM_F64,
    'F' => FORMAT_ITEM_F64,
    'f' => FORMAT_ITEM_F32,
    'H' => FORMAT_ITEM_HEX32,
    'h' => FORMAT_ITEM_HEX16,
    'i' => FORMAT_ITEM_DEC32S,
    'I' => FORMAT_ITEM_DEC64S,
    'L' => FORMAT_ITEM_DEC64S,
    'l' => FORMAT_ITEM_DEC64S,
    'O' => FORMAT_ITEM_OCT32,
    'o' => FORMAT_ITEM_OCT16,
    's' => FORMAT_ITEM_DEC16S,
    'X' => FORMAT_ITEM_HEX32,
    'x' => FORMAT_ITEM_HEX16
    ];

    let ignored_arg_opts: HashSet<_> = ['A', 'j', 'N', 'S', 'w'].iter().cloned().collect();

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
        }
        else if arg.starts_with("--") {
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
        }
        else if arg.starts_with("-") {
            let mut flags = arg.chars().skip(1);
            let mut format_spec = String::new();
            while let Some(c) = flags.next() {
                if expect_type_string {
                    format_spec.push(c);
                }
                else if ignored_arg_opts.contains(&c) {
                    break;
                }
                else if c=='t' {
                    expect_type_string = true;
                }
                else {
                    match known_formats.get(&c) {
                        None => {} // not every option is a format
                        Some(r) => {
                            formats.push(*r)
                        }
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
        formats.push(FORMAT_ITEM_OCT16); // 2 byte octal is the default
    }

    Ok(formats)
}

#[derive(PartialEq, Eq, Debug)]
enum ParseState {
    ExpectSize,     // expect optional size character like L for long.
    ExpectDecimal,  // expect optional additional digits, like for 16.
    ExpectDump,     // expect optional 'z'.
    Finished        // no more characters may appear.
}

fn parse_type_string(params: &String) -> Result<Vec<FormatterItemInfo>, String> {

    let type_chars: HashSet<_> = ['a', 'c'].iter().cloned().collect();
    let type_ints: HashSet<_> = ['d', 'o', 'u', 'x'].iter().cloned().collect();
    let type_floats: HashSet<_> = ['f'].iter().cloned().collect();
    let type_all: HashSet<_> =
            type_chars.iter()
            .chain(type_ints.iter())
            .chain(type_floats.iter())
            .collect();

    let mut formats = Vec::new();

    // first split a type string into parts refering a single type
    let mut type_parts = Vec::new();
    let mut s = String::new();
    for c in params.chars() {
        if type_all.contains(&c) {
            if !s.is_empty() {
                type_parts.push(s);
                s = String::new();
            }
            s.push(c);
        }
        else {
            if s.is_empty() {
                return Err(format!("unexpected char '{}' in format specification '{}'", c, params));
            }
            s.push(c);
        }
    }
    if !s.is_empty() {
        type_parts.push(s);
    }

    for format_type in type_parts.iter() {
        let mut chars=format_type.chars();

        let type_char = chars.next().unwrap();

        let mut parse_state = ParseState::ExpectSize;
        let mut decimal_size = String::new();
        let mut byte_size = 0u8;
        let mut show_ascii_dump = false;

        if type_chars.contains(&type_char) {
            parse_state = ParseState::ExpectDump;
        }

        loop {
            match chars.next() {
                None => break,
                Some('z') if parse_state != ParseState::Finished => {
                    show_ascii_dump = true;
                    parse_state = ParseState::Finished;
                },
                Some(d) if d.is_digit(10)
                        && (parse_state == ParseState::ExpectSize || parse_state == ParseState::ExpectDecimal) => {
                    decimal_size.push(d);
                    parse_state = ParseState::ExpectDecimal;
                },

                Some('C') if type_ints.contains(&type_char) && parse_state == ParseState::ExpectSize => {
                    byte_size = 1;
                    parse_state = ParseState::ExpectDump;
                },
                Some('S') if type_ints.contains(&type_char) && parse_state == ParseState::ExpectSize => {
                    byte_size = 2;
                    parse_state = ParseState::ExpectDump;
                },
                Some('I') if type_ints.contains(&type_char) && parse_state == ParseState::ExpectSize => {
                    byte_size = 4;
                    parse_state = ParseState::ExpectDump;
                },
                Some('L') if type_ints.contains(&type_char) && parse_state == ParseState::ExpectSize => {
                    byte_size = 8;
                    parse_state = ParseState::ExpectDump;
                },

                Some('F') if type_char == 'f' && parse_state == ParseState::ExpectSize => {
                    byte_size = 4;
                    parse_state = ParseState::ExpectDump;
                },
                Some('D') if type_char == 'f' && parse_state == ParseState::ExpectSize => {
                    byte_size = 8;
                    parse_state = ParseState::ExpectDump;
                },
                // Some('L') if type_char == 'f' => byte_size = 16, // TODO support f128

                Some(c) => {
                    return Err(format!("unexpected char '{}' in format specification '{}'", c, format_type));
                }
            }
        }

        if !decimal_size.is_empty() {
            byte_size=match decimal_size.parse() {
                Err(_) => return Err(format!("invalid number '{}' in format specification '{}'", decimal_size, format_type)),
                Ok(n) => n,
            }
        }

        match type_char {
            'a' => formats.push(FORMAT_ITEM_A),
            'c' => formats.push(FORMAT_ITEM_C),
            'd' => {
                formats.push(match byte_size {
                    1 => FORMAT_ITEM_DEC8S,
                    2 => FORMAT_ITEM_DEC16S,
                    4|0 => FORMAT_ITEM_DEC32S,
                    8 => FORMAT_ITEM_DEC64S,
                    _ => return Err(format!("invalid size '{}' in format specification '{}'", byte_size, format_type)),
                });
            },
            'o' => {
                formats.push(match byte_size {
                    1 => FORMAT_ITEM_OCT8,
                    2 => FORMAT_ITEM_OCT16,
                    4|0 => FORMAT_ITEM_OCT32,
                    8 => FORMAT_ITEM_OCT64,
                    _ => return Err(format!("invalid size '{}' in format specification '{}'", byte_size, format_type)),
                });
            },
            'u' => {
                formats.push(match byte_size {
                    1 => FORMAT_ITEM_DEC8U,
                    2 => FORMAT_ITEM_DEC16U,
                    4|0 => FORMAT_ITEM_DEC32U,
                    8 => FORMAT_ITEM_DEC64U,
                    _ => return Err(format!("invalid size '{}' in format specification '{}'", byte_size, format_type)),
                });
            },
            'x' => {
                formats.push(match byte_size {
                    1 => FORMAT_ITEM_HEX8,
                    2 => FORMAT_ITEM_HEX16,
                    4|0 => FORMAT_ITEM_HEX32,
                    8 => FORMAT_ITEM_HEX64,
                    _ => return Err(format!("invalid size '{}' in format specification '{}'", byte_size, format_type)),
                });
            },
            'f' => {
                formats.push(match byte_size {
                    4|0 => FORMAT_ITEM_F32,
                    8 => FORMAT_ITEM_F64,
                    _ => return Err(format!("invalid size '{}' in format specification '{}'", byte_size, format_type)),
                });
            },
            _ => unreachable!(),
        }

        if show_ascii_dump { /*TODO*/ }
    }

    Ok(formats)
}

#[allow(dead_code)]
pub fn parse_format_flags_str(args_str: &Vec<&'static str>) -> Result<Vec<FormatterItemInfo>, String> {
    let args = args_str.iter().map(|s| s.to_string()).collect();
    parse_format_flags(&args)
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
    parse_format_flags_str(&vec!("od", "--format=f2")).unwrap_err();
}

#[test]
fn test_long_format_a() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=a")).unwrap(),
       vec!(FORMAT_ITEM_A));
}

#[test]
fn test_long_format_cz() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "--format=cz")).unwrap(),
       vec!(FORMAT_ITEM_C)); // TODO 'z'
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
   assert_eq!(parse_format_flags_str(
       &vec!(
           "od",
           "--skip-bytes=2",
           "-vItu1z",
           "-N",
           "1000",
           "-xt",
           "acdx1",
           "--format=u2c",
           "--format",
           "f",
           "-xAx",
           "--",
           "-h",
           "--format=f8")).unwrap(),
       vec!(
           FORMAT_ITEM_DEC64S,  // I
           FORMAT_ITEM_DEC8U,   // tu1z
           FORMAT_ITEM_HEX16,   // x
           FORMAT_ITEM_A,       // ta
           FORMAT_ITEM_C,       // tc
           FORMAT_ITEM_DEC32S,  // td
           FORMAT_ITEM_HEX8,    // tx1
           FORMAT_ITEM_DEC16U,  // tu2
           FORMAT_ITEM_C,       // tc
           FORMAT_ITEM_F32,     // tf
           FORMAT_ITEM_HEX16,   // x
       ));
}
