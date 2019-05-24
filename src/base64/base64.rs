#![crate_name = "uu_base64"]

// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

#[macro_use]
extern crate uucore;
use uucore::encoding::Format;

mod base_common;

static SYNTAX: &str = "[OPTION]... [FILE]";
static SUMMARY: &str =
    "Base64 encode or decode FILE, or standard input, to standard output.";
static LONG_HELP: &str = "
 With no FILE, or when FILE is -, read standard input.

 The data are encoded as described for the base64 alphabet in RFC
 3548. When decoding, the input may contain newlines in addition
 to the bytes of the formal base64 alphabet. Use --ignore-garbage
 to attempt to recover from any other non-alphabet bytes in the
 encoded stream.
";

pub fn uumain(args: Vec<String>) -> i32 {
    base_common::execute(args, SYNTAX, SUMMARY, LONG_HELP, Format::Base64)
}
