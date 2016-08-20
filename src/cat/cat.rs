#![crate_name = "uu_cat"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: cat (GNU coreutils) 8.13 */

extern crate libc;

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::intrinsics::{copy_nonoverlapping};
use std::io::{stdout, stdin, stderr, Write, Read, Result};
use uucore::fs::is_stdin_interactive;

static SYNTAX: &'static str = "[OPTION]... [FILE]..."; 
static SUMMARY: &'static str = "Concatenate FILE(s), or standard input, to standard output
 With no FILE, or when FILE is -, read standard input."; 
static LONG_HELP: &'static str = ""; 

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("A", "show-all", "equivalent to -vET")
        .optflag("b", "number-nonblank",
                 "number nonempty output lines, overrides -n")
        .optflag("e", "", "equivalent to -vE")
        .optflag("E", "show-ends", "display $ at end of each line")
        .optflag("n", "number", "number all output lines")
        .optflag("s", "squeeze-blank", "suppress repeated empty output lines")
        .optflag("t", "", "equivalent to -vT")
        .optflag("T", "show-tabs", "display TAB characters as ^I")
        .optflag("v", "show-nonprinting",
                 "use ^ and M- notation, except for LF (\\n) and TAB (\\t)")
        .parse(args);

    let number_mode = if matches.opt_present("b") {
        NumberingMode::NumberNonEmpty
    } else if matches.opt_present("n") {
        NumberingMode::NumberAll
    } else {
        NumberingMode::NumberNone
    };
    let show_nonprint = matches.opts_present(&["A".to_owned(), "e".to_owned(),
                                              "t".to_owned(), "v".to_owned()]);
    let show_ends = matches.opts_present(&["E".to_owned(), "A".to_owned(),
                                          "e".to_owned()]);
    let show_tabs = matches.opts_present(&["A".to_owned(), "T".to_owned(),
                                          "t".to_owned()]);
    let squeeze_blank = matches.opt_present("s");
    let mut files = matches.free;
    if files.is_empty() {
        files.push("-".to_owned());
    }

    exec(files, number_mode, show_nonprint, show_ends, show_tabs, squeeze_blank);

    0
}

#[derive(Eq, PartialEq)]
enum NumberingMode {
    NumberNone,
    NumberNonEmpty,
    NumberAll,
}

fn write_lines(files: Vec<String>, number: NumberingMode, squeeze_blank: bool,
               show_ends: bool) {

    let mut line_counter: usize = 1;

    for (mut reader, interactive) in files.iter().filter_map(|p| open(&p[..])) {

        let mut in_buf  = [0; 1024 * 31];
        let mut out_buf = [0; 1024 * 64];
        let mut writer = UnsafeWriter::new(&mut out_buf[..], stdout());
        let mut at_line_start = true;
        let mut one_blank_kept = false;
        while let Ok(n) = reader.read(&mut in_buf) {
            if n == 0 { break }

            let in_buf = &in_buf[..n];
            let mut buf_pos = 0..n;
            loop {
                writer.possibly_flush();
                let pos = match buf_pos.next() {
                    Some(p) => p,
                    None => break,
                };
                if in_buf[pos] == '\n' as u8 {
                    if !at_line_start || !squeeze_blank || !one_blank_kept {
                        one_blank_kept = true;
                        if at_line_start && number == NumberingMode::NumberAll {
                            (write!(&mut writer, "{0:6}\t", line_counter)).unwrap();
                            line_counter += 1;
                        }
                        if show_ends {
                            writer.write_all(&['$' as u8]).unwrap();
                        }
                        writer.write_all(&['\n' as u8]).unwrap();
                        if interactive {
                            writer.flush().unwrap();
                        }
                    }
                    at_line_start = true;
                    continue;
                } else if one_blank_kept {
                    one_blank_kept = false;
                }
                if at_line_start && number != NumberingMode::NumberNone {
                    (write!(&mut writer, "{0:6}\t", line_counter)).unwrap();
                    line_counter += 1;
                }
                match in_buf[pos..].iter().position(|c| *c == '\n' as u8) {
                    Some(p) => {
                        writer.write_all(&in_buf[pos..pos + p]).unwrap();
                        if show_ends {
                            writer.write_all(&['$' as u8]).unwrap();
                        }
                        writer.write_all(&['\n' as u8]).unwrap();
                        if interactive {
                            writer.flush().unwrap();
                        }
                        buf_pos = pos + p + 1..n;
                        at_line_start = true;
                    },
                    None => {
                        writer.write_all(&in_buf[pos..]).unwrap();
                        at_line_start = false;
                        break;
                    }
                };
            }
        }
    }
}

fn write_bytes(files: Vec<String>, number: NumberingMode, squeeze_blank: bool,
               show_ends: bool, show_nonprint: bool, show_tabs: bool) {

    let mut line_counter: usize = 1;

    for (mut reader, interactive) in files.iter().filter_map(|p| open(&p[..])) {

        // Flush all 1024 iterations.
        let mut flush_counter = 0usize..1024;

        let mut in_buf  = [0; 1024 * 32];
        let mut out_buf = [0; 1024 * 64];
        let mut writer = UnsafeWriter::new(&mut out_buf[..], stdout());
        let mut at_line_start = true;
        while let Ok(n) = reader.read(&mut in_buf) {
            if n == 0 { break }

            for &byte in in_buf[..n].iter() {
                if flush_counter.next().is_none() {
                    writer.possibly_flush();
                    flush_counter = 0usize..1024;
                }
                if byte == '\n' as u8 {
                    if !at_line_start || !squeeze_blank {
                        if at_line_start && number == NumberingMode::NumberAll {
                            (write!(&mut writer, "{0:6}\t", line_counter)).unwrap();
                            line_counter += 1;
                        }
                        if show_ends {
                            writer.write_all(&['$' as u8]).unwrap();
                        }
                        writer.write_all(&['\n' as u8]).unwrap();
                        if interactive {
                            writer.flush().unwrap();
                        }
                    }
                    at_line_start = true;
                    continue;
                }
                if at_line_start && number != NumberingMode::NumberNone {
                    (write!(&mut writer, "{0:6}\t", line_counter)).unwrap();
                    line_counter += 1;
                    at_line_start = false;
                }
                // This code is slow because of the many branches. cat in glibc avoids
                // this by having the whole loop inside show_nonprint.
                if byte == '\t' as u8 {
                    if show_tabs {
                        writer.write_all("^I".as_bytes())
                    } else {
                        writer.write_all(&[byte])
                    }
                } else if show_nonprint {
                    let byte = match byte {
                        128 ... 255 => {
                            writer.write_all("M-".as_bytes()).unwrap();
                            byte - 128
                        },
                        _ => byte,
                    };
                    match byte {
                        0 ... 31 => writer.write_all(&['^' as u8, byte + 64]),
                        127      => writer.write_all(&['^' as u8, byte - 64]),
                        _        => writer.write_all(&[byte]),
                    }
                } else {
                    writer.write_all(&[byte])
                }.unwrap();
            }
        }
    }
}

fn write_fast(files: Vec<String>) {
    let mut writer = stdout();
    let mut in_buf = [0; 1024 * 64];

    for (mut reader, _) in files.iter().filter_map(|p| open(&p[..])) {
        while let Ok(n) = reader.read(&mut in_buf) {
            if n == 0 { break }
            // This interface is completely broken.
            writer.write_all(&in_buf[..n]).unwrap();
        }
    }
}

fn exec(files: Vec<String>, number: NumberingMode, show_nonprint: bool,
        show_ends: bool, show_tabs: bool, squeeze_blank: bool) {

    if show_nonprint || show_tabs {
        write_bytes(files, number, squeeze_blank, show_ends, show_nonprint, show_tabs);
    } else if number != NumberingMode::NumberNone || squeeze_blank || show_ends {
        write_lines(files, number, squeeze_blank, show_ends);
    } else {
        write_fast(files);
    }
    pipe_flush!();
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
        },
    }
}

struct UnsafeWriter<'a, W: Write> {
    inner: W,
    buf: &'a mut [u8],
    pos: usize,
    threshold: usize,
}

impl<'a, W: Write> UnsafeWriter<'a, W> {
    fn new(buf: &'a mut [u8], inner: W) -> UnsafeWriter<'a, W> {
        let threshold = buf.len()/2;
        UnsafeWriter {
            inner: inner,
            buf: buf,
            pos: 0,
            threshold: threshold,
        }
    }

    fn flush_buf(&mut self) -> Result<()> {
        if self.pos != 0 {
            let ret = self.inner.write(&self.buf[..self.pos]);
            self.pos = 0;
            match ret {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        } else {
            Ok(())
        }
    }

    fn possibly_flush(&mut self) {
        if self.pos > self.threshold {
            self.inner.write_all(&self.buf[..self.pos]).unwrap();
            self.pos = 0;
        }
    }
}

#[inline(never)]
fn fail() -> ! {
    panic!("assertion failed");
}

impl<'a, W: Write> Write for UnsafeWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let dst = &mut self.buf[self.pos..];
        let len = buf.len();
        if len > dst.len() {
            fail();
        }
        unsafe {
            copy_nonoverlapping(buf.as_ptr(), dst.as_mut_ptr(), len)
        }
        self.pos += len;
        Ok(len)
    }

    fn flush(&mut self) -> Result<()> {
        self.flush_buf().and_then(|()| self.inner.flush())
    }
}

impl<'a, W: Write> Drop for UnsafeWriter<'a, W> {
    fn drop(&mut self) {
        let _ = self.flush_buf();
    }
}
