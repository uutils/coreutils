#[link(name="cat", vers="1.0.0", author="Seldaek")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: cat (GNU coreutils) 8.13 */

extern mod extra;

use std::os;
use std::io::{stdin, stderr, stdout, Writer, Reader};
use extra::getopts::groups;

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        groups::optflag("A", "show-all", "equivalent to -vET"),
        groups::optflag("b", "number-nonblank", "number nonempty output lines, overrides -n"),
        groups::optflag("e", "", "equivalent to -vE"),
        groups::optflag("E", "show-ends", "display $ at end of each line"),
        groups::optflag("n", "number", "number all output lines"),
        groups::optflag("s", "squeeze-blank", "suppress repeated empty output lines"),
        groups::optflag("t", "", "equivalent to -vT"),
        groups::optflag("T", "show-tabs", "display TAB characters as ^I"),
        groups::optflag("v", "show-nonprinting", "use ^ and M- notation, except for LF (\\n) and TAB (\\t)"),
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            stderr().write_line("Invalid options");
            stderr().write_line(f.to_err_msg());
            os::set_exit_status(1);
            return
        }
    };
    if matches.opts_present([~"h", ~"help"]) {
        println("cat 1.0.0");
        println("");
        println("Usage:");
        println(format!("  {0:s} [OPTION]... [FILE]...", program));
        println("");
        print(groups::usage("Concatenate FILE(s), or standard input, to standard output.", opts));
        println("");
        println("With no FILE, or when FILE is -, read standard input.");
        return;
    }
    if matches.opts_present([~"V", ~"version"]) {
        println("cat 1.0.0");
        return;
    }

    let mut number_mode = NumberNone;
    if matches.opts_present([~"n", ~"number"]) {
        number_mode = NumberAll;
    }
    if matches.opts_present([~"b", ~"number-nonblank"]) {
        number_mode = NumberNonEmpty;
    }
    let show_nonprint = matches.opts_present([~"v", ~"show-nonprinting", ~"A", ~"show-all", ~"t", ~"e"]);
    let show_ends = matches.opts_present([~"E", ~"show-ends", ~"A", ~"show-all", ~"e"]);
    let show_tabs = matches.opts_present([~"T", ~"show-tabs",  ~"A", ~"show-all", ~"t"]);
    let squeeze_blank = matches.opts_present([~"s", ~"squeeze-blank"]);
    let mut files = matches.free;
    if files.is_empty() {
        files = ~[~"-"];
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

pub fn exec(files: ~[~str], number: NumberingMode, show_nonprint: bool, show_ends: bool, show_tabs: bool, squeeze_blank: bool) {
    let writer = stdout();

    if NumberNone != number || show_nonprint || show_ends || show_tabs || squeeze_blank {
        for path in files.iter() {
            let reader = match open(path.to_owned()) {
                Some(f) => f,
                None => { continue }
            };

            let mut counter: uint = 1;
            let mut at_line_start = true;
            let is_numbering = number == NumberAll || number == NumberNonEmpty;

            loop {
                let buf = reader.read_bytes(2);
                for byte in buf.iter() {
                    if at_line_start && (number == NumberAll || (number == NumberNonEmpty && !is_newline_char(*byte))) {
                        writer.write_str(format!("{0:6u}  ", counter));
                        counter += 1;
                        at_line_start = false;
                    }
                    if is_numbering && *byte == LF {
                        at_line_start = true;
                    }
                    if show_tabs && *byte == TAB {
                        writer.write(bytes!("^I"));
                    } else if show_ends && *byte == LF {
                        writer.write(bytes!("$\n"));
                    } else if show_nonprint && (*byte < 32 || *byte >= 127) && !is_newline_char(*byte) {
                        let mut byte = *byte;
                        if byte >= 128 {
                            writer.write(bytes!("M-"));
                            byte = byte - 128;
                        }
                        if byte < 32 {
                            writer.write(['^' as u8, byte + 64]);
                        } else if byte == 127 {
                            writer.write(['^' as u8, byte - 64]);
                        } else {
                            writer.write([byte]);
                        }
                    } else {
                        writer.write([*byte]);
                    }
                }

                if reader.eof() {
                    break;
                }
            }
        }

        return;
    }

    // passthru mode
    for path in files.iter() {
        let reader = match open(path.to_owned()) {
            Some(f) => f,
            None => { continue }
        };

        loop {
            writer.write(reader.read_bytes(100000));
            if reader.eof() {
                break;
            }
        }
    }
}

fn open(path: ~str) -> Option<@Reader> {
    if "-" == path {
        return Some(stdin());
    }

    match std::io::file_reader(&std::path::Path::new(path.as_slice())) {
        Ok(fd) => return Some(fd),
        Err(e) => {
            stderr().write_line(format!("cat: {0:s}: {1:s}", path, e));
            os::set_exit_status(1);
        }
    }

    None
}
