#![crate_id(name="cat", vers="1.0.0", author="Seldaek")]
#![feature(unsafe_destructor)]

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
use std::io::stdio::{stdout_raw, stdin_raw, stderr};
use std::io::{IoResult};
use std::ptr::{copy_nonoverlapping_memory};

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0).as_slice();
    let opts = [
        getopts::optflag("A", "show-all", "equivalent to -vET"),
        getopts::optflag("b", "number-nonblank",
                         "number nonempty output lines, overrides -n"),
        getopts::optflag("e", "", "equivalent to -vE"),
        getopts::optflag("E", "show-ends", "display $ at end of each line"),
        getopts::optflag("n", "number", "number all output lines"),
        getopts::optflag("s", "squeeze-blank", "suppress repeated empty output lines"),
        getopts::optflag("t", "", "equivalent to -vT"),
        getopts::optflag("T", "show-tabs", "display TAB characters as ^I"),
        getopts::optflag("v", "show-nonprinting",
                         "use ^ and M- notation, except for LF (\\n) and TAB (\\t)"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!("Invalid options\n{}", f)
    };
    if matches.opt_present("help") {
        println!("cat 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("");
        print(getopts::usage("Concatenate FILE(s), or standard input, to \
                             standard output.", opts).as_slice());
        println!("");
        println!("With no FILE, or when FILE is -, read standard input.");
        return 0;
    }
    if matches.opt_present("version") {
        println!("cat 1.0.0");
        return 0;
    }

    let mut number_mode = NumberNone;
    if matches.opt_present("n") {
        number_mode = NumberAll;
    }
    if matches.opt_present("b") {
        number_mode = NumberNonEmpty;
    }
    let show_nonprint = matches.opts_present(["A".to_string(), "e".to_string(),
                                              "t".to_string(), "v".to_string()]);
    let show_ends = matches.opts_present(["E".to_string(), "A".to_string(),
                                          "e".to_string()]);
    let show_tabs = matches.opts_present(["A".to_string(), "T".to_string(),
                                          "t".to_string()]);
    let squeeze_blank = matches.opt_present("s");
    let mut files = matches.free;
    if files.is_empty() {
        files.push("-".to_string());
    }

    exec(files, number_mode, show_nonprint, show_ends, show_tabs, squeeze_blank);

    0
}

#[deriving(Eq, PartialEq)]
enum NumberingMode {
    NumberNone,
    NumberNonEmpty,
    NumberAll,
}

fn write_lines(files: Vec<String>, number: NumberingMode, squeeze_blank: bool,
               show_ends: bool) {

    let mut line_counter: uint = 1;

    for path in files.iter() {
        let (mut reader, interactive) = match open(path.as_slice()) {
            Some(f) => f,
            None => continue,
        };

        let mut in_buf  = [0, .. 1024 * 31];
        let mut out_buf = [0, .. 1024 * 64];
        let mut writer = UnsafeWriter::new(out_buf.as_mut_slice(), stdout_raw());
        let mut at_line_start = true;
        loop {
            let n = match reader.read(in_buf) {
                Ok(n) if n != 0 => n,
                _ => break,
            };
            let in_buf = in_buf.slice_to(n);
            let mut buf_pos = range(0, n);
            loop {
                writer.possibly_flush();
                let pos = match buf_pos.next() {
                    Some(p) => p,
                    None => break,
                };
                if in_buf[pos] == '\n' as u8 {
                    if !at_line_start || !squeeze_blank {
                        if at_line_start && number == NumberAll {
                            (write!(&mut writer, "{0:6u}\t", line_counter)).unwrap();
                            line_counter += 1;
                        }
                        if show_ends {
                            writer.write_u8('$' as u8).unwrap();
                        }
                        writer.write_u8('\n' as u8).unwrap();
                        if interactive {
                            writer.flush().unwrap();
                        }
                    }
                    at_line_start = true;
                    continue;
                }
                if at_line_start && number != NumberNone {
                    (write!(&mut writer, "{0:6u}\t", line_counter)).unwrap();
                    line_counter += 1;
                }
                match in_buf.tailn(pos).iter().position(|c| *c == '\n' as u8) {
                    Some(p) => {
                        writer.write(in_buf.slice(pos, pos + p)).unwrap();
                        if show_ends {
                            writer.write_u8('$' as u8).unwrap();
                        }
                        writer.write_u8('\n' as u8).unwrap();
                        if interactive {
                            writer.flush().unwrap();
                        }
                        buf_pos = range(pos + p + 1, n);
                        at_line_start = true;
                    },
                    None => {
                        writer.write(in_buf.tailn(pos)).unwrap();
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

    let mut line_counter: uint = 1;

    for path in files.iter() {
        let (mut reader, interactive) = match open(path.as_slice()) {
            Some(f) => f,
            None => continue,
        };

        // Flush all 1024 iterations.
        let mut flush_counter = range(0u, 1024);

        let mut in_buf  = [0, .. 1024 * 32];
        let mut out_buf = [0, .. 1024 * 64];
        let mut writer = UnsafeWriter::new(out_buf.as_mut_slice(), stdout_raw());
        let mut at_line_start = true;
        loop {
            let n = match reader.read(in_buf) {
                Ok(n) if n != 0 => n,
                _ => break,
            };
            for &byte in in_buf.slice_to(n).iter() {
                if flush_counter.next().is_none() {
                    writer.possibly_flush();
                    flush_counter = range(0u, 1024);
                }
                if byte == '\n' as u8 {
                    if !at_line_start || !squeeze_blank {
                        if at_line_start && number == NumberAll {
                            (write!(&mut writer, "{0:6u}\t", line_counter)).unwrap();
                            line_counter += 1;
                        }
                        if show_ends {
                            writer.write_u8('$' as u8).unwrap();
                        }
                        writer.write_u8('\n' as u8).unwrap();
                        if interactive {
                            writer.flush().unwrap();
                        }
                    }
                    at_line_start = true;
                    continue;
                }
                if at_line_start && number != NumberNone {
                    (write!(&mut writer, "{0:6u}\t", line_counter)).unwrap();
                    line_counter += 1;
                    at_line_start = false;
                }
                // This code is slow because of the many branches. cat in glibc avoids
                // this by having the whole loop inside show_nonprint.
                if byte == '\t' as u8 {
                    if show_tabs {
                        writer.write_str("^I")
                    } else {
                        writer.write_u8(byte)
                    }
                } else if show_nonprint {
                    let byte = match byte {
                        128 .. 255 => {
                            writer.write_str("M-").unwrap();
                            byte - 128
                        },
                        _ => byte,
                    };
                    match byte {
                        0 .. 31 => writer.write(['^' as u8, byte + 64]),
                        127     => writer.write(['^' as u8, byte - 64]),
                        _       => writer.write_u8(byte),
                    }
                } else {
                    writer.write_u8(byte)
                }.unwrap();
            }
        }
    }
}

fn write_fast(files: Vec<String>) {
    let mut writer = stdout_raw();
    let mut in_buf = [0, .. 1024 * 64];

    for path in files.iter() {
        let (mut reader, _) = match open(path.as_slice()) {
            Some(x) => x,
            None => continue,
        };

        loop {
            match reader.read(in_buf) {
                Ok(n) if n != 0 => {
                    // This interface is completely broken.
                    writer.write(in_buf.slice_to(n)).unwrap();
                },
                _ => break
            }
        }
    }
}

fn exec(files: Vec<String>, number: NumberingMode, show_nonprint: bool,
        show_ends: bool, show_tabs: bool, squeeze_blank: bool) {

    if show_nonprint || show_tabs {
        write_bytes(files, number, squeeze_blank, show_ends, show_nonprint, show_tabs);
    } else if number != NumberNone || squeeze_blank || show_ends {
        write_lines(files, number, squeeze_blank, show_ends);
    } else {
        write_fast(files);
    }
}

fn open(path: &str) -> Option<(Box<Reader>, bool)> {
    if path == "-" {
        let stdin = stdin_raw();
        let interactive = stdin.isatty();
        return Some((box stdin as Box<Reader>, interactive));
    }

    match File::open(&std::path::Path::new(path)) {
        Ok(f) => Some((box f as Box<Reader>, false)),
        Err(e) => {
            (writeln!(stderr(), "cat: {0:s}: {1:s}", path, e.to_str())).unwrap();
            None
        },
    }
}

struct UnsafeWriter<'a, W> {
    inner: W,
    buf: &'a mut [u8],
    pos: uint,
    threshold: uint,
}

impl<'a, W: Writer> UnsafeWriter<'a, W> {
    fn new(buf: &'a mut [u8], inner: W) -> UnsafeWriter<'a, W> {
        let threshold = buf.len()/2;
        UnsafeWriter {
            inner: inner,
            buf: buf,
            pos: 0,
            threshold: threshold,
        }
    }

    fn flush_buf(&mut self) -> IoResult<()> {
        if self.pos != 0 {
            let ret = self.inner.write(self.buf.slice_to(self.pos));
            self.pos = 0;
            ret
        } else {
            Ok(())
        }
    }

    fn possibly_flush(&mut self) {
        if self.pos > self.threshold {
            self.inner.write(self.buf.slice_to(self.pos)).unwrap();
            self.pos = 0;
        }
    }
}

#[inline(never)]
fn fail() -> ! {
    fail!("assertion failed");
}

impl<'a, W: Writer> Writer for UnsafeWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        let dst = self.buf.mut_slice_from(self.pos);
        if buf.len() > dst.len() {
            fail();
        }
        unsafe {
            copy_nonoverlapping_memory(dst.as_mut_ptr(), buf.as_ptr(), buf.len())
        }
        self.pos += buf.len();
        Ok(())
    }

    fn flush(&mut self) -> IoResult<()> {
        self.flush_buf().and_then(|()| self.inner.flush())
    }
}

#[unsafe_destructor]
impl<'a, W: Writer> Drop for UnsafeWriter<'a, W> {
    fn drop(&mut self) {
        let _ = self.flush_buf();
    }
}

/* vim: set ai ts=4 sw=4 sts=4 et : */
