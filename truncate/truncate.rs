#[crate_id(name="truncate", vers="1.0.0", author="Arcterus")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::io::{stderr, io_error, File, Open, ReadWrite, Writer, SeekEnd, SeekSet};
use std::os;
use std::u64;
use extra::getopts::groups;

#[deriving(Eq)]
enum TruncateMode {
    Reference,
    Extend,
    Reduce,
    AtMost,
    AtLeast,
    RoundDown,
    RoundUp
}

fn main() {
    let args = os::args();
    let program = args[0].clone();

    let opts = ~[
        groups::optflag("c", "no-create", "do not create files that do not exist"),
        groups::optflag("o", "io-blocks", "treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)"),
        groups::optopt("r", "reference", "base the size of each file on the size of RFILE", "RFILE"),
        groups::optopt("s", "size", "set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified", "SIZE"),
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit")
    ];
    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer, "{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }
    };

    if matches.opt_present("help") {
        println!("truncate 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... FILE...", program);
        println!("");
        print!("{}", groups::usage("Shrink or extend the size of each file to the specified size.", opts));
        println!("");
        println!("SIZE is an integer with an optional prefix and optional unit.");
        println!("The available units (K, M, G, T, P, E, Z, and Y) use the following format:");
        println!("\t'KB' =>           1000 (kilobytes)");
        println!("\t'K'  =>           1024 (kibibytes)");
        println!("\t'MB' =>      1000*1000 (megabytes)");
        println!("\t'M'  =>      1024*1024 (mebibytes)");
        println!("\t'GB' => 1000*1000*1000 (gigabytes)");
        println!("\t'G'  => 1024*1024*1024 (gibibytes)");
        println!("SIZE may also be prefixed by one of the following to adjust the size of each");
        println!("file based on its current size:");
        println!("\t'+'  => extend by");
        println!("\t'-'  => reduce by");
        println!("\t'<'  => at most");
        println!("\t'>'  => at least");
        println!("\t'/'  => round down to multiple of");
        println!("\t'%'  => round up to multiple of");
    } else if matches.opt_present("version") {
        println!("truncate 1.0.0");
    } else if matches.free.is_empty() {
        writeln!(&mut stderr() as &mut Writer, "Missing an argument");
        writeln!(&mut stderr() as &mut Writer,
                 "For help, try '{0:s} --help'", program);
        os::set_exit_status(1);
    } else {
        let no_create = matches.opt_present("no-create");
        let io_blocks = matches.opt_present("io-blocks");
        let reference = matches.opt_str("reference");
        let size = matches.opt_str("size");
        if reference.is_none() && size.is_none() {
            writeln!(&mut stderr() as &mut Writer, "You must specify either --reference or --size.");
            os::set_exit_status(1);
        } else {
            truncate(no_create, io_blocks, reference, size, matches.free);
        }
    }
}

fn truncate(no_create: bool, io_blocks: bool, reference: Option<~str>, size: Option<~str>, filenames: ~[~str]) {
    let (refsize, mode) = match reference {
        Some(rfilename) => {
            io_error::cond.trap(|err| {
                writeln!(&mut stderr() as &mut Writer, "{}", err.to_str());
                os::set_exit_status(1);
            }).inside(|| {
                let mut rfile = File::open(&Path::new(rfilename.clone()));
                rfile.seek(0, SeekEnd);
                (rfile.tell(), Reference)
            })
        }
        None => {
            match parse_size(size.unwrap()) {
                Ok(szpair) => szpair,
                Err(()) => return
            }
        }
    };
    for filename in filenames.iter() {
        let filename: &str = *filename;
        let path = Path::new(filename);
        if !path.exists() && !no_create {
            io_error::cond.trap(|_| {
                writeln!(&mut stderr() as &mut Writer,
                         "Failed to create the file '{}'", filename);
                os::set_exit_status(1);
            }).inside(|| {
                File::create(&path);
            });
        }
        io_error::cond.trap(|err| {
            writeln!(&mut stderr() as &mut Writer, "{}", err.to_str());
        }).inside(|| {
            match File::open_mode(&path, Open, ReadWrite) {
                Some(mut file) => {
                    file.seek(0, SeekEnd);
                    let fsize = file.tell();
                    file.seek(0, SeekSet);
                    let tsize = match mode {
                        Reference => refsize,
                        Extend => fsize + refsize,
                        Reduce => fsize - refsize,
                        AtMost => if fsize > refsize { refsize } else { fsize },
                        AtLeast => if fsize < refsize { refsize } else { fsize },
                        RoundDown => fsize - fsize % refsize,
                        RoundUp => fsize + fsize % refsize
                    };
                    file.truncate(tsize as i64);
                }
                None => {
                    writeln!(&mut stderr() as &mut Writer,
                             "Failed to open the file '{}'", filename);
                    os::set_exit_status(1);
                }
            }
        });
    }
}

fn parse_size(size: ~str) -> Result<(u64, TruncateMode), ()> {
    let mut err = false;
    let mode = match size.char_at(0) {
        '+' => Extend,
        '-' => Reduce,
        '<' => AtMost,
        '>' => AtLeast,
        '/' => RoundDown,
        '*' => RoundUp,
        _ => Reference /* assume that the size is just a number */
    };
    let bytes = {
        let mut slice =
            if mode == Reference {
                let size: &str = size;
                size
            } else {
                size.slice_from(1)
            };
        if slice.char_at(slice.len() - 1).is_alphabetic() {
            slice = slice.slice_to(slice.len() - 1);
            if slice.len() > 0 && slice.char_at(slice.len() - 1).is_alphabetic() {
                slice = slice.slice_to(slice.len() - 1);
            }
        }
        slice
    }.bytes().to_owned_vec();
    let mut number = match u64::parse_bytes(bytes, 10) {
        Some(num) => num,
        None => {
            writeln!(&mut stderr() as &mut Writer,
                     "'{}' is not a valid number.", size);
            os::set_exit_status(1);
            err = true;
            0
        }
    };
    if !err && size.char_at(size.len() - 1).is_alphabetic() {
        number *= match size.char_at(size.len() - 1).to_ascii().to_upper().to_char() {
            'B' => match size.char_at(size.len() - 2).to_ascii().to_upper().to_char() {
                'K' => 1000,
                'M' => 1000 * 1000,
                'G' => 1000 * 1000 * 1000,
                'T' => 1000 * 1000 * 1000 * 1000,
                'P' => 1000 * 1000 * 1000 * 1000 * 1000,
                'E' => 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                'Z' => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                'Y' => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                letter => {
                    writeln!(&mut stderr() as &mut Writer,
                             "'{}B' is not a valid suffix.", letter);
                    os::set_exit_status(1);
                    err = true;
                    1
                }
            },
            'K' => 1024,
            'M' => 1024 * 1024,
            'G' => 1024 * 1024 * 1024,
            'T' => 1024 * 1024 * 1024 * 1024,
            'P' => 1024 * 1024 * 1024 * 1024 * 1024,
            'E' => 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            'Z' => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            'Y' => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            letter => {
                writeln!(&mut stderr() as &mut Writer,
                         "'{}' is not a valid suffix.", letter);
                os::set_exit_status(1);
                err = true;
                1
            }
        };
    }
    if err {
        Err(())
    } else {
        Ok((number, mode))
    }
}

