#[crate_id(name="du", vers="1.0.0", author="Derek Chiang")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[allow(uppercase_variables)];
#[feature(macro_rules)];

extern crate getopts;
extern crate sync;
extern crate time;

use std::os;
use std::io::{stderr, fs, FileStat, TypeDirectory};
use std::option::Option;
use std::path::Path;
use time::Timespec;
use sync::{Arc, Future};

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "du";
static VERSION: &'static str = "1.0.0";

struct Options {
    all: bool,
    program_name: ~str,
    max_depth: Option<uint>,
    total: bool,
    separate_dirs: bool,
}

// this takes `my_stat` to avoid having to stat files multiple times.
fn du(path: &Path, mut my_stat: FileStat,
      options: Arc<Options>, depth: uint) -> ~[Arc<FileStat>] {
    let mut stats = ~[];
    let mut futures = ~[];

    if my_stat.kind == TypeDirectory {
        let read = match fs::readdir(path) {
            Ok(read) => read,
            Err(e) => {
                writeln!(&mut stderr(), "{}: cannot read directory ‘{}‘: {}",
                         options.program_name, path.display(), e);
                return ~[Arc::new(my_stat)]
            }
        };

        for f in read.move_iter() {
            let this_stat = safe_unwrap!(fs::lstat(&f));
            if this_stat.kind == TypeDirectory {
                let oa_clone = options.clone();
                futures.push(Future::spawn(proc() { du(&f, this_stat, oa_clone, depth + 1) }))
            } else {
                my_stat.size += this_stat.size;
                my_stat.unstable.blocks += this_stat.unstable.blocks;
                if options.all {
                    stats.push(Arc::new(this_stat))
                }
            }
        }
    }

    for future in futures.mut_iter() {
        for stat in future.get().move_rev_iter() {
            if !options.separate_dirs && stat.path.dir_path() == my_stat.path {
                my_stat.size += stat.size;
                my_stat.unstable.blocks += stat.unstable.blocks;
            }
            if options.max_depth == None || depth < options.max_depth.unwrap() {
                stats.push(stat.clone());
            }
        }
    }

    stats.push(Arc::new(my_stat));

    return stats;
}

fn main() {
    let args = os::args();
    let program = args[0].as_slice();
    let opts = ~[
        // In task
        getopts::optflag("a", "all", " write counts for all files, not just directories"),
        // In main
        getopts::optflag("", "apparent-size", "print apparent sizes,  rather  than  disk  usage;
            although  the apparent  size is usually smaller, it may be larger due to holes
            in ('sparse') files, internal  fragmentation,  indirect  blocks, and the like"),
        // In main
        getopts::optopt("B", "block-size", "scale sizes  by  SIZE before printing them.
            E.g., '-BM' prints sizes in units of 1,048,576 bytes.  See SIZE format below.",
            "SIZE"),
        // In main
        getopts::optflag("b", "bytes", "equivalent to '--apparent-size --block-size=1'"),
        // In main
        getopts::optflag("c", "total", "produce a grand total"),
        // In task
        // getopts::optflag("D", "dereference-args", "dereference only symlinks that are listed
        //     on the command line"),
        // In main
        // getopts::optopt("", "files0-from", "summarize disk usage of the NUL-terminated file
        //                   names specified in file F;
        //                   If F is - then read names from standard input", "F"),
        // // In task
        // getopts::optflag("H", "", "equivalent to --dereference-args (-D)"),
        // In main
        getopts::optflag("h", "human-readable", "print sizes in human readable format (e.g., 1K 234M 2G)"),
        // In main
        getopts::optflag("", "si", "like -h, but use powers of 1000 not 1024"),
        // In main
        getopts::optflag("k", "", "like --block-size=1K"),
        // In task
        getopts::optflag("l", "count-links", "count sizes many times if hard linked"),
        // // In main
        getopts::optflag("m", "", "like --block-size=1M"),
        // // In task
        // getopts::optflag("L", "dereference", "dereference all symbolic links"),
        // // In task
        // getopts::optflag("P", "no-dereference", "don't follow any symbolic links (this is the default)"),
        // // In main
        getopts::optflag("0", "null", "end each output line with 0 byte rather than newline"),
        // In main
        getopts::optflag("S", "separate-dirs", "do not include size of subdirectories"),
        // In main
        getopts::optflag("s", "summarize", "display only a total for each argument"),
        // // In task
        // getopts::optflag("x", "one-file-system", "skip directories on different file systems"),
        // // In task
        // getopts::optopt("X", "exclude-from", "exclude files that match any pattern in FILE", "FILE"),
        // // In task
        // getopts::optopt("", "exclude", "exclude files that match PATTERN", "PATTERN"),
        // In main
        getopts::optopt("d", "max-depth", "print the total for a directory (or file, with --all)
            only if it is N or fewer levels below the command
            line argument;  --max-depth=0 is the same as --summarize", "N"),
        // In main
        getopts::optflagopt("", "time", "show time of the last modification of any file in the
            directory, or any of its subdirectories.  If WORD is given, show time as WORD instead of modification time:
            atime, access, use, ctime or status", "WORD"),
        // In main
        getopts::optopt("", "time-style", "show times using style STYLE:
            full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'", "STYLE"),
        getopts::optflag("", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            show_error!(1, "Invalid options\n{}", f.to_err_msg());
            return
        }
    };

    if matches.opt_present("help") {
        println!("{program} {version} - estimate file space usage

Usage
  {program} [OPTION]... [FILE]...
  {program} [OPTION]... --files0-from=F

{usage}

Display  values  are  in  units  of  the  first  available  SIZE from
--block-size,  and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environ‐
ment variables.  Otherwise, units default to  1024  bytes  (or  512  if
POSIXLY_CORRECT is set).

SIZE  is  an  integer and optional unit (example: 10M is 10*1024*1024).
Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB, ...  (pow‐
ers of 1000).",
                 program = program,
                 version = VERSION,
                 usage = getopts::usage("Summarize disk usage of each FILE, recursively for directories.", opts));
        return
    } else if matches.opt_present("version") {
        println!("{} version: {}", program, VERSION);
        return
    }

    let summarize = matches.opt_present("summarize");

    let max_depth_str = matches.opt_str("max-depth");
    let max_depth = max_depth_str.as_ref().and_then(|s| from_str::<uint>(*s));
    match (max_depth_str, max_depth) {
        (Some(ref s), _) if summarize => {
            println!("{}: warning: summarizing conflicts with --max-depth={:s}", program, *s);
            return
        }
        (Some(ref s), None) => {
            println!("{}: invalid maximum depth '{:s}'", program, *s);
            return
        }
        (Some(_), Some(_)) | (None, _) => { /* valid */ }
    }

    let options = Options {
        all: matches.opt_present("all"),
        program_name: program.to_owned(),
        max_depth: max_depth,
        total: matches.opt_present("total"),
        separate_dirs: matches.opt_present("S"),
    };

    let strs = if matches.free.is_empty() {vec!(~"./")} else {matches.free.clone()};

    let options_arc = Arc::new(options);

    let MB = match matches.opt_present("si") {
        true => 1000 * 1000,
        false => 1024 * 1024,
    };
    let KB = match matches.opt_present("si") {
        true => 1000,
        false => 1024,
    };

    let block_size = match matches.opt_str("block-size") {
        Some(s) => {
            let mut found_number = false;
            let mut found_letter = false;
            let mut numbers = ~[];
            let mut letters = ~[];
            for c in s.chars() {
                if found_letter && c.is_digit() || !found_number && !c.is_digit() {
                    println!("{}: invalid --block-size argument '{}'", program, s);
                    return
                } else if c.is_digit() {
                    found_number = true;
                    numbers.push(c as u8);
                } else if c.is_alphabetic() {
                    found_letter = true;
                    letters.push(c);
                }
            }
            let number = std::uint::parse_bytes(numbers, 10).unwrap();
            let multiple = match std::str::from_chars(letters).as_slice() {
                "K" => 1024, "M" => 1024 * 1024, "G" => 1024 * 1024 * 1024,
                "T" => 1024 * 1024 * 1024 * 1024, "P" => 1024 * 1024 * 1024 * 1024 * 1024,
                "E" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                "Z" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                "Y" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                "KB" => 1000, "MB" => 1000 * 1000, "GB" => 1000 * 1000 * 1000,
                "TB" => 1000 * 1000 * 1000 * 1000, "PB" => 1000 * 1000 * 1000 * 1000 * 1000,
                "EB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                "ZB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                "YB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                _ => {
                    println!("{}: invalid --block-size argument '{}'", program, s); return
                }
            };
            number * multiple
        },
        None => 1024
    };

    let convert_size = |size: u64| -> ~str {
        if matches.opt_present("human-readable") || matches.opt_present("si") {
            if size > MB {
                format!("{:.1f}M", (size as f64) / (MB as f64))
            } else if size > KB {
                format!("{:.1f}K", (size as f64) / (KB as f64))
            } else {
                format!("{}B", size)
            }
        } else if matches.opt_present("k") {
            format!("{}", ((size as f64) / (KB as f64)).ceil())
        } else if matches.opt_present("m") {
            format!("{}", ((size as f64) / (MB as f64)).ceil())
        } else {
            format!("{}", ((size as f64) / (block_size as f64)).ceil())
        }
    };

    let time_format_str = match matches.opt_str("time-style") {
        Some(s) => {
            match s.as_slice() {
                "full-iso" => "%Y-%m-%d %H:%M:%S.%f %z",
                "long-iso" => "%Y-%m-%d %H:%M",
                "iso" => "%Y-%m-%d",
                _ => {
                    println!("{program}: invalid argument '{}' for 'time style'
Valid arguments are:
- 'full-iso'
- 'long-iso'
- 'iso'
Try '{program} --help' for more information.", s, program = program);
                    return
                }
            }
        },
        None => "%Y-%m-%d %H:%M"
    };

    let line_separator = match matches.opt_present("0") {
        true => "\0",
        false => "\n",
    };

    let mut grand_total = 0;
    for path_str in strs.move_iter() {
        let path = Path::new(path_str);
        let stat = safe_unwrap!(fs::lstat(&path));
        let iter = du(&path, stat, options_arc.clone(), 0).move_iter();
        let (_, len) = iter.size_hint();
        let len = len.unwrap();
        for (index, stat) in iter.enumerate() {
            let size = match matches.opt_present("apparent-size") {
                true => stat.unstable.nlink * stat.size,
                // C's stat is such that each block is assume to be 512 bytes
                // See: http://linux.die.net/man/2/stat
                false => stat.unstable.blocks * 512,
            };
            if matches.opt_present("time") {
                let time_str = {
                    let (secs, nsecs) = {
                        let time = match matches.opt_str("time") {
                            Some(s) => match s.as_slice() {
                                "accessed" => stat.accessed,
                                "created" => stat.created,
                                "modified" => stat.modified,
                                _ => {
                                    println!("{program}: invalid argument 'modified' for '--time'
    Valid arguments are:
      - 'accessed', 'created', 'modified'
    Try '{program} --help' for more information.", program = program);
                                    return
                                }
                            },
                            None => stat.modified
                        };
                        ((time / 1000) as i64, (time % 1000 * 1000000) as i32)
                    };
                    let time_spec = Timespec::new(secs, nsecs);
                    time::at(time_spec).strftime(time_format_str)
                };
                print!("{:<10} {:<30} {}", convert_size(size), time_str, stat.path.display());
            } else {
                print!("{:<10} {}", convert_size(size), stat.path.display());
            }
            print!("{}", line_separator);
            if options_arc.total && index == (len - 1) {
                // The last element will be the total size of the the path under
                // path_str.  We add it to the grand total.
                grand_total += size;
            }
        }
    }

    if options_arc.total {
        print!("{:<10} total", convert_size(grand_total));
        print!("{}", line_separator);
    }
}
