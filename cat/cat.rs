#![crate_id(name="cat", vers="1.0.0", author="Seldaek")]
#![feature(managed_boxes)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: cat (GNU coreutils) 8.13 */

extern crate getopts;

use std::os;
use std::io::{print, File};
use std::io::stdio::{stdout_raw, stdin_raw};
use std::io::{BufferedWriter};

fn main() {
    let args: Vec<StrBuf> = os::args().iter().map(|x| x.to_strbuf()).collect();
    let program = args.get(0).as_slice();
    let opts = ~[
        getopts::optflag("A", "show-all", "equivalent to -vET"),
        getopts::optflag("b", "number-nonblank", "number nonempty output lines, overrides -n"),
        getopts::optflag("e", "", "equivalent to -vE"),
        getopts::optflag("E", "show-ends", "display $ at end of each line"),
        getopts::optflag("n", "number", "number all output lines"),
        getopts::optflag("s", "squeeze-blank", "suppress repeated empty output lines"),
        getopts::optflag("t", "", "equivalent to -vT"),
        getopts::optflag("T", "show-tabs", "display TAB characters as ^I"),
        getopts::optflag("v", "show-nonprinting", "use ^ and M- notation, except for LF (\\n) and TAB (\\t)"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!("Invalid options\n{}", f.to_err_msg())
    };
    if matches.opt_present("help") {
        println!("cat 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("");
        print(getopts::usage("Concatenate FILE(s), or standard input, to standard output.", opts).as_slice());
        println!("");
        println!("With no FILE, or when FILE is -, read standard input.");
        return;
    }
    if matches.opt_present("version") {
        println!("cat 1.0.0");
        return;
    }

    let mut number_mode = NumberNone;
    if matches.opt_present("number") {
        number_mode = NumberAll;
    }
    if matches.opt_present("number-nonblank") {
        number_mode = NumberNonEmpty;
    }
    let show_nonprint = matches.opts_present(["show-nonprinting".to_strbuf(), "show-all".to_strbuf(), "t".to_strbuf(), "e".to_strbuf()]);
    let show_ends = matches.opts_present(["show-ends".to_strbuf(), "show-all".to_strbuf(), "e".to_strbuf()]);
    let show_tabs = matches.opts_present(["show-tabs".to_strbuf(), "show-all".to_strbuf(), "t".to_strbuf()]);
    let squeeze_blank = matches.opt_present("squeeze-blank");
    let mut files = matches.free;
    if files.is_empty() {
        files = vec!("-".to_strbuf());
    }

    exec(files, number_mode, show_nonprint, show_ends, show_tabs, squeeze_blank);
}

#[deriving(Eq)]
pub enum NumberingMode {
    NumberNone,
    NumberNonEmpty,
    NumberAll,
}

static TAB: u8 = '\t' as u8;
#[allow(dead_code)]
static CR: u8 = '\r' as u8;
static LF: u8 = '\n' as u8;

#[cfg(windows)]
fn is_newline_char(byte: u8) -> bool {
    byte == LF || byte == CR
}

#[cfg(unix)]
fn is_newline_char(byte: u8) -> bool {
    byte == LF
}

pub fn exec(files: Vec<StrBuf>, number: NumberingMode, show_nonprint: bool, show_ends: bool, show_tabs: bool, squeeze_blank: bool) {

    if NumberNone != number || show_nonprint || show_ends || show_tabs || squeeze_blank {
        let mut counter: uint = 1;
        let is_numbering = number == NumberAll || number == NumberNonEmpty;

        for path in files.iter() {
            let mut reader = match open(path.as_slice()) {
                Some(f) => f,
                None => { continue }
            };

            let mut writer = BufferedWriter::with_capacity(1024 * 8, stdout_raw());
            let mut at_line_start = true;
            let mut buf = ~[0, .. 1024 * 4];
            loop {
                // reading from a TTY seems to raise a condition on
                // EOF, rather than return Some(0) like a file.
                match reader.read(buf) {
                    Ok(n) if n != 0 => {
                        for &byte in buf.slice_to(n).iter() {
                            if at_line_start && (number == NumberAll || (number == NumberNonEmpty && !is_newline_char(byte))) {
                                (write!(&mut writer as &mut Writer, "{0:6u}\t", counter)).unwrap();
                                counter += 1;
                                at_line_start = false;
                            }
                            if is_numbering && byte == LF {
                                at_line_start = true;
                            }
                            if show_tabs && byte == TAB {
                                writer.write(bytes!("^I")).unwrap();
                            } else if show_ends && byte == LF {
                                writer.write(bytes!("$\n")).unwrap();
                            } else if show_nonprint && (byte < 32 || byte >= 127) && !is_newline_char(byte) {
                                let mut byte = byte;
                                if byte >= 128 {
                                    writer.write(bytes!("M-")).unwrap();
                                    byte = byte - 128;
                                }
                                if byte < 32 {
                                    writer.write(['^' as u8, byte + 64]).unwrap();
                                } else if byte == 127 {
                                    writer.write(['^' as u8, byte - 64]).unwrap();
                                } else {
                                    writer.write_u8(byte).unwrap();
                                }
                            } else {
                                writer.write_u8(byte).unwrap();
                            }
                        }
                    },
                    _ => break
                }
            }
        }
        return;
    }

    let mut writer = stdout_raw();
    let mut buf = ~[0, .. 1024 * 64];
    // passthru mode
    for path in files.iter() {
        let mut reader = match open(path.as_slice()) {
            Some(f) => f,
            None => { continue }
        };

        loop {
            // reading from a TTY seems to raise a condition on EOF,
            // rather than return Some(0) like a file.
            match reader.read(buf) {
                Ok(n) if n != 0 => {
                    writer.write(buf.slice_to(n)).unwrap();
                }, _ => break
            }
        }
    }
}

fn open(path: &str) -> Option<Box<Reader>> {
    if "-" == path {
        return Some(box stdin_raw() as Box<Reader>);
    }

    match File::open(&std::path::Path::new(path.as_slice())) {
        Ok(fd) => return Some(box fd as Box<Reader>),
        Err(e) => fail!("cat: {0:s}: {1:s}", path, e.to_str())
    }
}
/* vim: set ai ts=4 sw=4 sts=4 et : */
