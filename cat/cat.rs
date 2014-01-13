#[crate_id(name="cat", vers="1.0.0", author="Seldaek")];
#[feature(managed_boxes)];

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
use std::io::{print, stdin, stderr, stdout, File, result};
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
            writeln!(&mut stderr() as &mut Writer,
                   "Invalid options\n{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }
    };
    if matches.opt_present("help") {
        println!("cat 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("");
        print(groups::usage("Concatenate FILE(s), or standard input, to standard output.", opts));
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
    let show_nonprint = matches.opts_present([~"show-nonprinting", ~"show-all", ~"t", ~"e"]);
    let show_ends = matches.opts_present([~"show-ends", ~"show-all", ~"e"]);
    let show_tabs = matches.opts_present([~"show-tabs",  ~"show-all", ~"t"]);
    let squeeze_blank = matches.opt_present("squeeze-blank");
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
    let mut writer = stdout();

    if NumberNone != number || show_nonprint || show_ends || show_tabs || squeeze_blank {
        let mut counter: uint = 1;
        let is_numbering = number == NumberAll || number == NumberNonEmpty;

        for path in files.iter() {
            let mut reader = match open(path.to_owned()) {
                Some(f) => f,
                None => { continue }
            };

            let mut at_line_start = true;
            let mut buf = [0, .. 2];
            loop {
                // reading from a TTY seems to raise a condition on
                // EOF, rather than return Some(0) like a file.
                match result(|| reader.read(buf)) {
                    Ok(Some(n)) if n != 0 => {
                        for byte in buf.slice_to(n).iter() {
                            if at_line_start && (number == NumberAll || (number == NumberNonEmpty && !is_newline_char(*byte))) {
                                write!(&mut writer as &mut Writer, "{0:6u}\t", counter);
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
                    }
                    _ => break
                }
            }
        }

        return;
    }

    let mut buf = [0, .. 100000];
    // passthru mode
    for path in files.iter() {
        let mut reader = match open(path.to_owned()) {
            Some(f) => f,
            None => { continue }
        };

        loop {
            // reading from a TTY seems to raise a condition on EOF,
            // rather than return Some(0) like a file.
            match result(|| reader.read(buf)) {
                Ok(Some(n)) if n != 0 => writer.write(buf.slice_to(n)),
                _ => break
            }
        }
    }
}

fn open(path: ~str) -> Option<~Reader> {
    if "-" == path {
        return Some(~stdin() as ~Reader);
    }

    match result(|| File::open(&std::path::Path::new(path.as_slice()))) {
        Ok(fd) => return Some(~fd as ~Reader),
        Err(e) => {
            writeln!(&mut stderr() as &mut Writer,
                     "cat: {0:s}: {1:s}", path, e.to_str());
            os::set_exit_status(1);
        }
    }

    None
}
