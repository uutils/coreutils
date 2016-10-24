#![crate_name = "uu_cat"]

// This file is part of the uutils coreutils package.
//
// (c) Jordi Boggiano <j.boggiano@seld.be>
// (c) Evgeniy Klyuchikov <evgeniy.klyuchikov@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
#[macro_use]
extern crate uucore;

// last synced with: cat (GNU coreutils) 8.13
use std::fs::File;
use std::io::{stdout, stdin, stderr, Write, Read, BufWriter};
use uucore::fs::is_stdin_interactive;

static SYNTAX: &'static str = "[OPTION]... [FILE]...";
static SUMMARY: &'static str = "Concatenate FILE(s), or standard input, to standard output
 With no FILE, or when FILE is -, read standard input.";
static LONG_HELP: &'static str = "";

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("A", "show-all", "equivalent to -vET")
        .optflag("b",
                 "number-nonblank",
                 "number nonempty output lines, overrides -n")
        .optflag("e", "", "equivalent to -vE")
        .optflag("E", "show-ends", "display $ at end of each line")
        .optflag("n", "number", "number all output lines")
        .optflag("s", "squeeze-blank", "suppress repeated empty output lines")
        .optflag("t", "", "equivalent to -vT")
        .optflag("T", "show-tabs", "display TAB characters as ^I")
        .optflag("v",
                 "show-nonprinting",
                 "use ^ and M- notation, except for LF (\\n) and TAB (\\t)")
        .parse(args);

    let number_mode = if matches.opt_present("b") {
        NumberingMode::NumberNonEmpty
    } else if matches.opt_present("n") {
        NumberingMode::NumberAll
    } else {
        NumberingMode::NumberNone
    };
    let show_nonprint =
        matches.opts_present(&["A".to_owned(), "e".to_owned(), "t".to_owned(), "v".to_owned()]);
    let show_ends = matches.opts_present(&["E".to_owned(), "A".to_owned(), "e".to_owned()]);
    let show_tabs = matches.opts_present(&["A".to_owned(), "T".to_owned(), "t".to_owned()]);
    let squeeze_blank = matches.opt_present("s");
    let mut files = matches.free;
    if files.is_empty() {
        files.push("-".to_owned());
    }

    if show_tabs || show_nonprint || show_ends || squeeze_blank ||
       number_mode != NumberingMode::NumberNone {
        write_lines(files,
                    number_mode,
                    squeeze_blank,
                    show_ends,
                    show_tabs,
                    show_nonprint);
    } else {
        write_fast(files);
    }
    pipe_flush!();

    0
}

#[derive(PartialEq)]
enum NumberingMode {
    NumberNone,
    NumberNonEmpty,
    NumberAll,
}

fn open(path: &str) -> Option<(Box<Read>, bool)> {
    if path == "-" {
        let stdin = stdin();
        return Some((Box::new(stdin) as Box<Read>, is_stdin_interactive()));
    }

    match File::open(path) {
        Ok(f) => Some((Box::new(f) as Box<Read>, false)),
        Err(e) => {
            (writeln!(&mut stderr(), "cat: {0}: {1}", path, e.to_string())).unwrap();
            None
        }
    }
}

fn write_fast(files: Vec<String>) {
    let mut writer = stdout();
    let mut in_buf = [0; 1024 * 64];

    for (mut reader, _) in files.iter().filter_map(|p| open(&p[..])) {
        while let Ok(n) = reader.read(&mut in_buf) {
            if n == 0 {
                break;
            }
            writer.write_all(&in_buf[..n]).unwrap();
        }
    }
}

fn write_lines(files: Vec<String>,
               number: NumberingMode,
               squeeze_blank: bool,
               show_ends: bool,
               show_tabs: bool,
               show_nonprint: bool) {
    // initialize end of line
    let end_of_line = if show_ends {
        "$\n".as_bytes()
    } else {
        "\n".as_bytes()
    };
    // initialize tab simbol
    let tab = if show_tabs {
        "^I".as_bytes()
    } else {
        "\t".as_bytes()
    };
    let mut line_counter: usize = 1;

    for (mut reader, interactive) in files.iter().filter_map(|p| open(&p[..])) {
        let mut in_buf = [0; 1024 * 31];
        let mut writer = BufWriter::with_capacity(1024 * 64, stdout());
        let mut at_line_start = true;
        let mut one_blank_kept = false;
        while let Ok(n) = reader.read(&mut in_buf) {
            if n == 0 {
                break;
            }
            let in_buf = &in_buf[..n];
            let mut pos = 0;
            while pos < n {
                // skip empty lines enumerating them if needed
                if in_buf[pos] == '\n' as u8 {
                    if !at_line_start || !squeeze_blank || !one_blank_kept {
                        one_blank_kept = true;
                        if at_line_start && number == NumberingMode::NumberAll {
                            (write!(&mut writer, "{0:6}\t", line_counter)).unwrap();
                            line_counter += 1;
                        }
                        writer.write_all(end_of_line).unwrap();
                        if interactive {
                            writer.flush().unwrap();
                        }
                    }
                    at_line_start = true;
                    pos += 1;
                    continue;
                }
                one_blank_kept = false;
                if at_line_start && number != NumberingMode::NumberNone {
                    (write!(&mut writer, "{0:6}\t", line_counter)).unwrap();
                    line_counter += 1;
                }

                // print to end of line or end of buffer
                let offset = if show_nonprint {
                    write_nonprint_to_end(&in_buf[pos..], &mut writer, tab)
                } else if show_tabs {
                    write_tab_to_end(&in_buf[pos..], &mut writer)
                } else {
                    write_to_end(&in_buf[pos..], &mut writer)
                };
                // end of buffer?
                if offset == 0 {
                    at_line_start = false;
                    break;
                }
                // print suitable end of line
                writer.write_all(end_of_line).unwrap();
                if interactive {
                    writer.flush().unwrap();
                }
                at_line_start = true;
                pos += offset;
            }
        }
    }
}

// write***_to_end methods
// Write all simbols till end of line or end of buffer
// Return the number of written bytes - 1 or 0 if the end of buffer is reached
fn write_to_end<W: Write>(in_buf: &[u8], writer: &mut W) -> usize {
    match in_buf.iter().position(|c| *c == '\n' as u8) {
        Some(p) => {
            writer.write_all(&in_buf[..p]).unwrap();
            p + 1
        }
        None => {
            writer.write_all(in_buf).unwrap();
            0
        }
    }
}

fn write_tab_to_end<W: Write>(in_buf: &[u8], writer: &mut W) -> usize {
    match in_buf.iter().position(|c| *c == '\n' as u8 || *c == '\t' as u8) {
        Some(p) => {
            writer.write_all(&in_buf[..p]).unwrap();
            if in_buf[p] == '\n' as u8 {
                p + 1
            } else {
                writer.write_all("^I".as_bytes()).unwrap();
                write_tab_to_end(&in_buf[p + 1..], writer)
            }
        }
        None => {
            writer.write_all(in_buf).unwrap();
            0
        }
    }
}

fn write_nonprint_to_end<W: Write>(in_buf: &[u8], writer: &mut W, tab: &[u8]) -> usize {
    let mut count = 0;

    for byte in in_buf.iter().map(|c| *c) {
        if byte == '\n' as u8 {
            break;
        }
        match byte {
                9 => writer.write_all(tab),
                0...8 | 10...31 => writer.write_all(&['^' as u8, byte + 64]),
                32...126 => writer.write_all(&[byte]),
                127 => writer.write_all(&['^' as u8, byte - 64]),
                128...159 => writer.write_all(&['M' as u8, '-' as u8, '^' as u8, byte - 64]),
                160...254 => writer.write_all(&['M' as u8, '-' as u8, byte - 128]),
                _ => writer.write_all(&['M' as u8, '-' as u8, '^' as u8, 63]),
            }
            .unwrap();
        count += 1;
    }
    if count != in_buf.len() { count + 1 } else { 0 }
}