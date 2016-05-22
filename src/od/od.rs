#![crate_name = "uu_od"]

// This file is part of the uutils coreutils package.
//
// (c) Ben Hirsch <benhirsch24@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

extern crate getopts;

use std::fs::File;
use std::io::Read;
use std::io::BufReader;
use std::io::Write;
use std::io;
use std::str;
use std::mem;

#[derive(Debug)]
enum Radix {
    Decimal,
    Hexadecimal,
    Octal,
    Binary,
}

struct OdFormat {
    itembytes: usize,
    writer: fn(u64, usize),
    offmarg: usize,
}

// TODO: use some sort of byte iterator, instead of passing bytes in u64
fn print_item_oct(p: u64, itembytes: usize) {
    let itemwidth = 3 * itembytes;
    let itemspace = 4 * itembytes - itemwidth;

    print!("{:>itemspace$}{:0width$o}",
           "",
           p,
           width = itemwidth,
           itemspace = itemspace);
}

fn print_item_hex(p: u64, itembytes: usize) {
    let itemwidth = 2 * itembytes;
    let itemspace = 4 * itembytes - itemwidth;

    print!("{:>itemspace$}{:0width$x}",
           "",
           p,
           width = itemwidth,
           itemspace = itemspace);
}


fn sign_extend(item: u64, itembytes: usize) -> i64{
	// https://graphics.stanford.edu/~seander/bithacks.html#VariableSignExtend
	unsafe{ 
		let b = 8 * itembytes; // number of bits representing the number in p
		let m =  mem::transmute::<u64,i64>(1u64 << (b - 1));
		let x =  mem::transmute::<u64,i64>(item) & (mem::transmute::<u64,i64>(1u64 << b) - 1);
		let r = (x ^ m) - m;
		r 
	}
}


fn print_item_dec_s(p: u64, itembytes: usize) {
    // sign extend
    let s = sign_extend(p,itembytes);
    print!("{:totalwidth$}", s, totalwidth = 4 * itembytes);
}
fn print_item_dec_u(p: u64, itembytes: usize) {
    print!("{:totalwidth$}", p, totalwidth = 4 * itembytes);
}

// TODO: multi-byte chars
// Quoth the man page: Multi-byte characters are displayed in the area corresponding to the first byte of the character. The remaining bytes are shown as `**'.

static A_CHRS : [&'static str; 160]  = 
["nul",   "soh",   "stx",   "etx",   "eot",   "enq",   "ack",   "bel",  
 "bs",    "ht",   "nl",     "vt",    "ff",    "cr",    "so",    "si",  
 "dle",   "dc1",   "dc2",   "dc3",   "dc4",   "nak",   "syn",   "etb",  
 "can",   "em",   "sub",   "esc",    "fs",    "gs",    "rs",    "us",  
 "sp",     "!",     "\"",     "#",     "$",     "%",     "&",     "'",  
  "(",     ")",     "*",     "+",     ",",     "-",     ".",     "/",  
  "0",     "1",     "2",     "3",     "4",     "5",     "6",     "7",  
  "8",     "9",     ":",     ";",     "<",     "=",     ">",     "?",  
  "@",     "A",     "B",     "C",     "D",     "E",     "F",     "G",  
  "H",     "I",     "J",     "K",     "L",     "M",     "N",     "O",  
  "P",     "Q",     "R",     "S",     "T",     "U",     "V",     "W",  
  "X",     "Y",     "Z",     "[",     "\\",    "]",     "^",     "_",  
  "`",     "a",     "b",     "c",     "d",     "e",     "f",     "g",  
  "h",     "i",     "j",     "k",     "l",     "m",     "n",     "o",  
  "p",     "q",     "r",     "s",     "t",     "u",     "v",     "w",  
  "x",     "y",     "z",     "{",     "|",     "}",     "~",   "del",  
 "80",    "81",    "82",    "83",    "84",    "85",    "86",    "87",  
 "88",    "89",    "8a",    "8b",    "8c",    "8d",    "8e",    "8f",  
 "90",    "91",    "92",    "93",    "94",    "95",    "96",    "97",  
 "98",    "99",    "9a",    "9b",    "9c",    "9d",    "9e",    "9f"];

fn print_item_a(p: u64, _: usize) {
    // itembytes == 1
    let b = (p & 0xff) as u8;
    print!("{:>4}", A_CHRS.get(b as usize).unwrap_or(&"?") // XXX od dose not actually do this, it just prints the byte
  );
}


static C_CHRS : [&'static str; 127]  = [
"\\0",   "001",   "002",   "003",   "004",   "005",   "006",    "\\a",  
"\\b",    "\\t",  "\\n",   "\\v",    "\\f",    "\\r",   "016",   "017",  
"020",   "021",   "022",   "023",   "024",   "025",   "026",   "027",  
"030",   "031",   "032",   "033",   "034",   "035",   "036",   "037",  
  " ",   "!",     "\"",     "#",     "$",     "%",     "&",     "'",  
  "(",     ")",     "*",     "+",     ",",     "-",     ".",     "/",  
  "0",     "1",     "2",     "3",     "4",     "5",     "6",     "7",  
  "8",     "9",     ":",     ";",     "<",     "=",     ">",     "?",  
  "@",     "A",     "B",     "C",     "D",     "E",     "F",     "G",  
  "H",     "I",     "J",     "K",     "L",     "M",     "N",     "O",  
  "P",     "Q",     "R",     "S",     "T",     "U",     "V",     "W",  
  "X",     "Y",     "Z",     "[",     "\\",     "]",     "^",     "_",  
  "`",     "a",     "b",     "c",     "d",     "e",     "f",     "g",  
  "h",     "i",     "j",     "k",     "l",     "m",     "n",     "o",  
  "p",     "q",     "r",     "s",     "t",     "u",     "v",     "w",  
  "x",     "y",     "z",     "{",     "|",     "}",     "~" ];


fn print_item_c(p: u64, _: usize) {
    // itembytes == 1
    let b = (p & 0xff) as usize;

    if b < C_CHRS.len() {
        match C_CHRS.get(b as usize) {
            Some(s) => print!("{:>4}", s),
            None => print!("{:>4}", b),
        }
    }
}


// Input sources
#[derive(Debug)]
enum InputSource<'a> {
    FileName(&'a str),
    Stdin,
}

impl<'b> MultifileReader<'b> {
    fn new<'a>(fnames: &'a [InputSource]) -> MultifileReader<'a> {
        let mut mf = MultifileReader {
            ni: fnames.iter(),
            curr_file: None, // normally this means done; call next_file()
            any_err: false,
        };
        mf.next_file();
        return mf;
    }

    fn next_file(&mut self) {
        // loop retries with subsequent files if err - normally 'loops' once
        loop {
            match self.ni.next() {
                None => {
                    self.curr_file = None;
                    return;
                }
                Some(input) => {
                    match *input {
                        InputSource::Stdin => {
                            self.curr_file = Some(Box::new(BufReader::new(std::io::stdin())));
                            return;
                        }
                        InputSource::FileName(fname) => {
                            match File::open(fname) {
                                Ok(f) => {
                                    self.curr_file = Some(Box::new(BufReader::new(f)));
                                    return;
                                }
                                Err(e) => {
                                    // If any file can't be opened,
                                    // print an error at the time that the file is needed,
                                    // then move on the the next file.
                                    // This matches the behavior of the original `od`
                                    let _ =
                                        writeln!(&mut std::io::stderr(), "od: '{}': {}", fname, e);
                                    self.any_err = true
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fill buf with bytes read from the list of files
    // Returns Ok(<number of bytes read>)
    // Handles io errors itself, thus always returns OK
    // Fills the provided buffer completely, unless it has run out of input.
    // If any call returns short (< buf.len()), all subsequent calls will return Ok<0>
    fn f_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut xfrd = 0;
        // while buffer we are filling is not full.. May go thru several files.
        'fillloop: while xfrd < buf.len() {
            match self.curr_file {
                None => break,
                Some(ref mut curr_file) => {
                    loop {
                        // stdin may return on 'return' (enter), even though the buffer isn't full.
                        xfrd += match curr_file.read(&mut buf[xfrd..]) {
                            Ok(0) => break, 
                            Ok(n) => n,
                            Err(e) => panic!("file error: {}", e),
                        };
                        if xfrd == buf.len() {
                            // transferred all that was asked for.
                            break 'fillloop;
                        }
                    }
                }
            }
            self.next_file();
        }
        Ok(xfrd)
    }
}

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt("A",
                "address-radix",
                "Select the base in which file offsets are printed.",
                "RADIX");
    opts.optopt("j",
                "skip-bytes",
                "Skip bytes input bytes before formatting and writing.",
                "BYTES");
    opts.optopt("N",
                "read-bytes",
                "limit dump to BYTES input bytes",
                "BYTES");
    opts.optopt("S",
                "strings",
                ("output strings of at least BYTES graphic chars. 3 is assumed when \
                 BYTES is not specified."),
                "BYTES");
    opts.optopt("t", "format", "select output format or formats", "TYPE");
    opts.optflag("v",
                 "output-duplicates",
                 "do not use * to mark line suppression");
    opts.optopt("w",
                "width",
                ("output BYTES bytes per output line. 32 is implied when BYTES is not \
                 specified."),
                "BYTES");
    opts.optflag("h", "help", "display this help and exit.");
    opts.optflag("", "version", "output version information and exit.");

    opts.optflag("a", "", "named characters, ignoring high-order bit");
    opts.optflag("b", "", "octal bytes");
    opts.optflag("c", "", "ASCII characters or backslash escapes");
    opts.optflag("d", "", "unsigned decimal 2-byte units");
    opts.optflag("o", "", "unsigned decimal 2-byte units");

    opts.optflag("I", "", "decimal 2-byte units");
    opts.optflag("L", "", "decimal 2-byte units");
    opts.optflag("i", "", "decimal 2-byte units");
    
    opts.optflag("O", "", "octal 4-byte units");
    opts.optflag("s", "", "decimal 4-byte units");
    
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("Invalid options\n{}", f),
    };

    let input_offset_base = match parse_radix(matches.opt_str("A")) {
        Ok(r) => r,
        Err(f) => panic!("Invalid -A/--address-radix\n{}", f),
    };

    // Gather up file names - args which don't start with '-'

    let stdnionly = [InputSource::Stdin];
    let inputs = args[1..]
        .iter()
        .filter_map(|w| match w as &str {  
            "--" => Some(InputSource::Stdin), 
            o if o.starts_with("-") => None,
            x => Some(InputSource::FileName(x)),
        })
        .collect::<Vec<_>>();;
    let inputs = if inputs.len() == 0 {
        &stdnionly[..]
    } else {
        &inputs[..]
    };

    let flags = args[1..]
        .iter()
        .filter_map(|w| match w as &str {  
            "--" => None, 
            o if o.starts_with("-") => Some(&o[1..]),
            _ => None,
        })
        .collect::<Vec<_>>();;

    // At the moment, char (-a & -c)formats need the driver to set up a
    // line by inserting a different # of of spaces at the start.
    struct OdFormater {
        writer: fn(p: u64, itembytes: usize),
        offmarg: usize,
    };
    let oct = OdFormater {
        writer: print_item_oct,
        offmarg: 2,
    };
    let hex = OdFormater {
        writer: print_item_hex,
        offmarg: 2,
    };
    let dec_u = OdFormater {
        writer: print_item_dec_u,
        offmarg: 2,
    };
    let dec_s = OdFormater {
        writer: print_item_dec_s,
        offmarg: 2,
    };
    let a_char = OdFormater {
        writer: print_item_a,
        offmarg: 1,
    };
    let c_char = OdFormater {
        writer: print_item_c,
        offmarg: 1,
    };

    fn mkfmt(itembytes: usize, fmtspec: &OdFormater) -> OdFormat {
        OdFormat {
            itembytes: itembytes,
            writer: fmtspec.writer,
            offmarg: fmtspec.offmarg,
        }
    }

    let known_formats = hashmap![
		"a" => (1, &a_char),
		"B" => (2, &oct) ,
		"b" => (1, &oct),
		"c" => (1, &c_char),
		"D" => (4, &dec_u),
// TODO: support floats
//		"e" => (8, &flo64),
//		"F" => (8, &flo64),
//		"F" => (4, &flo32),
		"H" => (4, &hex),
		"X" => (4, &hex) ,
		"o" => (2, &oct),
		"x" => (2, &hex),
		"h" => (2, &hex),

		"I" => (2, &dec_s),
		"L" => (2, &dec_s), 
		"i" => (2, &dec_s),

		"O" => (4, &oct),
		"s" => (2, &dec_u)
	];

    // TODO: -t fmts

    let mut formats = Vec::new();

    for flag in flags.iter() {
        match known_formats.get(flag) {
            // This should be caught above anyway not every option is a format
            None => {}
            Some(r) => {
                let (itembytes, fmtspec) = *r;
                formats.push(mkfmt(itembytes, fmtspec))
            }
        }
    }

    if formats.is_empty() {
        formats.push(mkfmt(2, &oct)); // 2 byte octal is the default
    }

    odfunc(&input_offset_base, &inputs, &formats[..])
}

const LINEBYTES: usize = 16;
const WORDBYTES: usize = 2;

struct MultifileReader<'a> {
    ni: std::slice::Iter<'a, InputSource<'a>>,
    curr_file: Option<Box<io::Read>>,
    any_err: bool,
}

fn odfunc(input_offset_base: &Radix, fnames: &[InputSource], formats: &[OdFormat]) -> i32 {

    let mut mf = MultifileReader::new(fnames);
    let mut addr = 0;
    let bytes = &mut [b'\x00'; LINEBYTES];
    loop {
        // print each line data (or multi-format raster of several lines describing the same data).

        print_with_radix(input_offset_base, addr); // print offset
		// if printing in multiple formats offset is printed only once

        match mf.f_read(bytes) {
            Ok(0) => {
                print!("\n");
                break;
            }
            Ok(n) => {
                let mut first = true; // First line of a multi-format raster.
                for f in formats {
                    if !first {
                        // this takes the space of the file offset on subsequent
                        // lines of multi-format rasters.
                        print!("       ");
                    }
                    first = false;
                    print!("{:>width$}", "", width = f.offmarg);// 4 spaces after offset - we print 2 more before each word

                    for b in 0..n / f.itembytes {
                        let mut p: u64 = 0;
                        for i in 0..f.itembytes {
                            p |= (bytes[(f.itembytes * b) + i] as u64) << (8 * i);
                        }
                        (f.writer)(p, f.itembytes);
                    }
                    // not enough byte for a whole element, this should only happen on the last line.
                    if n % f.itembytes != 0 {
                        let b = n / f.itembytes;
                        let mut p2: u64 = 0;
                        for i in 0..(n % f.itembytes) {
                            p2 |= (bytes[(f.itembytes * b) + i] as u64) << (8 * i);
                        }
                        (f.writer)(p2, f.itembytes);
                    }
                    // Add extra spaces to pad out the short, presumably last, line.
                    if n < LINEBYTES {
                        // calc # of items we did not print, must be short at least WORDBYTES to be missing any.
                        let words_short = (LINEBYTES - n) / WORDBYTES;
                        // XXX this is running short for -c & -a
                        print!("{:>width$}", "", width = (words_short) * (6 + 2));
                    }
                    print!("\n");
                }
                addr += n;
            }
            Err(_) => {
                break;
            }
        };
    }
    if mf.any_err {
        1
    } else {
        0
    }
}

fn parse_radix(radix_str: Option<String>) -> Result<Radix, &'static str> {
    match radix_str {
        None => Ok(Radix::Octal),
        Some(s) => {
            let st = s.into_bytes();
            if st.len() != 1 {
                Err("Radix must be one of [d, o, b, x]\n")
            } else {
                let radix: char = *(st.get(0)
                    .expect("byte string of length 1 lacks a \
                             0th elem")) as char;
                match radix {
                    'd' => Ok(Radix::Decimal),
                    'x' => Ok(Radix::Hexadecimal),
                    'o' => Ok(Radix::Octal),
                    'b' => Ok(Radix::Binary),
                    _ => Err("Radix must be one of [d, o, b, x]\n"),
                }
            }
        }
    }
}

fn print_with_radix(r: &Radix, x: usize) {
    // TODO(keunwoo): field widths should be based on sizeof(x), or chosen dynamically based on the
    // expected range of address values.  Binary in particular is not great here.
    match *r {
        Radix::Decimal => print!("{:07}", x),
        Radix::Hexadecimal => print!("{:07X}", x),
        Radix::Octal => print!("{:07o}", x),
        Radix::Binary => print!("{:07b}", x),
    }
}
