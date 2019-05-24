// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

#![crate_name = "uu_base32"]

#[macro_use]
extern crate uucore;
use uucore::encoding::Format;

#[path = "../base64/base_common.rs"]
mod base_common;

static SYNTAX: &str = "[OPTION]... [FILE]";
static SUMMARY: &str =
    "Base32 encode or decode FILE, or standard input, to standard output.";
static LONG_HELP: &str = "
 With no FILE, or when FILE is -, read standard input.

 The data are encoded as described for the base32 alphabet in RFC
 4648. When decoding, the input may contain newlines in addition
 to the bytes of the formal base32 alphabet. Use --ignore-garbage
 to attempt to recover from any other non-alphabet bytes in the
 encoded stream.
";

pub fn uumain(args: Vec<String>) -> i32 {
    base_common::execute(args, SYNTAX, SUMMARY, LONG_HELP, Format::Base32)
}
