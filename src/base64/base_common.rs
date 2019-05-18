// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

use uucore;
use uucore::encoding::{wrap_print, Data, Format};

use std::fs::File;
use std::io::{stdin, BufReader, Read};
use std::path::Path;

pub fn execute(
    args: Vec<String>,
    syntax: &str,
    summary: &str,
    long_help: &str,
    format: Format,
) -> i32 {
    let matches = new_coreopts!(syntax, summary, long_help)
        .optflag("d", "decode", "decode data")
        .optflag(
            "i",
            "ignore-garbage",
            "when decoding, ignore non-alphabetic characters",
        )
        .optopt(
            "w",
            "wrap",
            "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)",
            "COLS",
        )
        .parse(args);
    
    let line_wrap = matches.opt_str("wrap").map(|s| {
        match s.parse() {
            Ok(n) => n,
            Err(e) => {
                crash!(1, "invalid wrap size: ‘{}’: {}", s, e);
            }
        }
    });
    let ignore_garbage = matches.opt_present("ignore-garbage");
    let decode = matches.opt_present("decode");

    if matches.free.len() > 1 {
        disp_err!("extra operand ‘{}’", matches.free[0]);
        return 1;
    }

    if matches.free.is_empty() || &matches.free[0][..] == "-" {
        let stdin_raw = stdin();
        handle_input(&mut stdin_raw.lock(), format, line_wrap, ignore_garbage, decode);
    } else {
        let path = Path::new(matches.free[0].as_str());
        let file_buf = safe_unwrap!(File::open(&path));
        let mut input = BufReader::new(file_buf);
        handle_input(&mut input, format, line_wrap, ignore_garbage, decode);
    };

    0
}

fn handle_input<R: Read>(
    input: &mut R,
    format: Format,
    line_wrap: Option<usize>,
    ignore_garbage: bool,
    decode: bool,
) {
    let mut data = Data::new(input, format)
        .ignore_garbage(ignore_garbage);
    if let Some(wrap) = line_wrap {
        data = data.line_wrap(wrap);
    }

    if !decode {
        let encoded = data.encode();
        wrap_print(&data, encoded);
    } else {
        match data.decode() {
            Ok(s) => print!("{}", String::from_utf8(s).unwrap()),
            Err(_) => crash!(1, "invalid input"),
        }
    }
}