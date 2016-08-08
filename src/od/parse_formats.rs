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

pub fn parse_format_flags(args: &Vec<String>) -> Vec<FormatterItemInfo> {
    // Gather up format flags, we don't use getopts becase we need keep them in order.
    let flags = args[1..]
        .iter()
        .filter_map(|w| match w as &str {
            "--" => None,
            o if o.starts_with("-") => Some(&o[1..]),
            _ => None,
        })
        .collect::<Vec<_>>();

    // TODO: -t fmts
    let known_formats = hashmap![
        "a" => FORMAT_ITEM_A,
        "B" => FORMAT_ITEM_OCT16,
        "b" => FORMAT_ITEM_OCT8,
        "c" => FORMAT_ITEM_C,
        "D" => FORMAT_ITEM_DEC32U,
        "d" => FORMAT_ITEM_DEC16U,
        "e" => FORMAT_ITEM_F64,
        "F" => FORMAT_ITEM_F64,
        "f" => FORMAT_ITEM_F32,
        "H" => FORMAT_ITEM_HEX32,
        "X" => FORMAT_ITEM_HEX32,
        "o" => FORMAT_ITEM_OCT16,
        "x" => FORMAT_ITEM_HEX16,
        "h" => FORMAT_ITEM_HEX16,

        "I" => FORMAT_ITEM_DEC16S,
        "L" => FORMAT_ITEM_DEC16S,
        "i" => FORMAT_ITEM_DEC16S,

        "O" => FORMAT_ITEM_OCT32,
        "s" => FORMAT_ITEM_DEC16U
    ];

    let mut formats = Vec::new();

    for flag in flags.iter() {
        match known_formats.get(flag) {
            None => {} // not every option is a format
            Some(r) => {
                formats.push(*r)
            }
        }
    }

    if formats.is_empty() {
        formats.push(FORMAT_ITEM_OCT16); // 2 byte octal is the default
    }

    formats
}
