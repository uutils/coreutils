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
pub fn parse_format_flags(args: &Vec<String>) -> Vec<FormatterItemInfo> {
    // Gather up format flags, we don't use getopts becase we need keep them in order.

    // TODO: -t fmts
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

    while let Some(arg) = arg_iter.next() {
        if arg.starts_with("--") {
            if arg.len() == 2 {
                break;
            }
        }
        else if arg.starts_with("-") {
            let mut flags = arg.chars().skip(1);
            while let Some(c) = flags.next() {
                if ignored_arg_opts.contains(&c) {
                    break;
                }
                match known_formats.get(&c) {
                    None => {} // not every option is a format
                    Some(r) => {
                        formats.push(*r)
                    }
                }
            }
        }
    }

    if formats.is_empty() {
        formats.push(FORMAT_ITEM_OCT16); // 2 byte octal is the default
    }

    formats
}

#[allow(dead_code)]
pub fn parse_format_flags_str(args_str: &Vec<&'static str>) -> Vec<FormatterItemInfo> {
    let args = args_str.iter().map(|s| s.to_string()).collect();
    parse_format_flags(&args)
}

#[test]
fn test_no_options() {
    assert_eq!(parse_format_flags_str(
        &vec!("od")),
        vec!(FORMAT_ITEM_OCT16));
}

#[test]
fn test_one_option() {
    assert_eq!(parse_format_flags_str(
        &vec!("od", "-F")),
        vec!(FORMAT_ITEM_F64));
}

#[test]
fn test_two_separate_options() {
    assert_eq!(parse_format_flags_str(
        &vec!("od", "-F", "-x")),
        vec!(FORMAT_ITEM_F64, FORMAT_ITEM_HEX16));
}

#[test]
fn test_two_combined_options() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-Fx")),
       vec!(FORMAT_ITEM_F64, FORMAT_ITEM_HEX16));
}

#[test]
fn test_ignore_non_format_parameters() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-d", "-Ax")),
       vec!(FORMAT_ITEM_DEC16U));
}

#[test]
fn test_ignore_separate_parameters() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-I", "-A", "x")),
       vec!(FORMAT_ITEM_DEC64S));
}

#[test]
fn test_ignore_trailing_vals() {
   assert_eq!(parse_format_flags_str(
       &vec!("od", "-D", "--", "-x")),
       vec!(FORMAT_ITEM_DEC32U));
}
