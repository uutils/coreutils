#![crate_name = "du"]
#![feature(collections, core, io, libc, path, rustc_private, std_misc, unicode)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![allow(non_snake_case)]

extern crate getopts;
extern crate libc;
extern crate time;

use std::old_io::{stderr, fs, FileStat, FileType};
use std::num::Float;
use std::option::Option;
use std::path::Path;
use std::sync::{Arc, Future};
use time::Timespec;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "du";
static VERSION: &'static str = "1.0.0";

struct Options {
    all: bool,
    program_name: String,
    max_depth: Option<usize>,
    total: bool,
    separate_dirs: bool,
}

struct Stat {
    path: Path,
    fstat: FileStat,
}
// this takes `my_stat` to avoid having to stat files multiple times.
fn du(path: &Path, mut my_stat: Stat,
      options: Arc<Options>, depth: usize) -> Vec<Arc<Stat>> {
    let mut stats = vec!();
    let mut futures = vec!();

    if my_stat.fstat.kind == FileType::Directory {
        let read = match fs::readdir(path) {
            Ok(read) => read,
            Err(e) => {
                safe_writeln!(&mut stderr(), "{}: cannot read directory ‘{}‘: {}",
                              options.program_name, path.display(), e);
                return vec!(Arc::new(my_stat))
            }
        };

        for f in read.into_iter() {
            let this_stat = Stat{path: f.clone(), fstat: safe_unwrap!(fs::lstat(&f))};
            if this_stat.fstat.kind == FileType::Directory {
                let oa_clone = options.clone();
                futures.push(Future::spawn(move || { du(&f, this_stat, oa_clone, depth + 1) }))
            } else {
                my_stat.fstat.size += this_stat.fstat.size;
                my_stat.fstat.unstable.blocks += this_stat.fstat.unstable.blocks;
                if options.all {
                    stats.push(Arc::new(this_stat))
                }
            }
        }
    }

    for future in futures.iter_mut() {
        for stat in future.get().into_iter().rev() {
            if !options.separate_dirs && stat.path.dir_path() == my_stat.path {
                my_stat.fstat.size += stat.fstat.size;
                my_stat.fstat.unstable.blocks += stat.fstat.unstable.blocks;
            }
            if options.max_depth == None || depth < options.max_depth.unwrap() {
                stats.push(stat.clone());
            }
        }
    }

    stats.push(Arc::new(my_stat));

    stats
}

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].as_slice();
    let opts = [
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

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            show_error!("Invalid options\n{}", f);
            return 1;
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
                 usage = getopts::usage("Summarize disk usage of each FILE, recursively for directories.", &opts));
        return 0;
    } else if matches.opt_present("version") {
        println!("{} version: {}", program, VERSION);
        return 0;
    }

    let summarize = matches.opt_present("summarize");

    let max_depth_str = matches.opt_str("max-depth");
    let max_depth = max_depth_str.as_ref().and_then(|s| s.parse::<usize>().ok());
    match (max_depth_str, max_depth) {
        (Some(ref s), _) if summarize => {
            show_error!("summarizing conflicts with --max-depth={}", *s);
            return 1;
        }
        (Some(ref s), None) => {
            show_error!("invalid maximum depth '{}'", *s);
            return 1;
        }
        (Some(_), Some(_)) | (None, _) => { /* valid */ }
    }

    let options = Options {
        all: matches.opt_present("all"),
        program_name: program.to_string(),
        max_depth: max_depth,
        total: matches.opt_present("total"),
        separate_dirs: matches.opt_present("S"),
    };

    let strs = if matches.free.is_empty() {vec!("./".to_string())} else {matches.free.clone()};

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
            let mut numbers = String::new(); 
            let mut letters = String::new(); 
            for c in s.as_slice().chars() {
                if found_letter && c.is_digit(10) || !found_number && !c.is_digit(10) {
                    show_error!("invalid --block-size argument '{}'", s);
                    return 1;
                } else if c.is_digit(10) {
                    found_number = true;
                    numbers.push(c);
                } else if c.is_alphabetic() {
                    found_letter = true;
                    letters.push(c);
                }
            }
            let number = numbers.parse::<usize>().unwrap();
            let multiple = match letters.as_slice() {
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
                    show_error!("invalid --block-size argument '{}'", s);
                    return 1;
                }
            };
            number * multiple
        },
        None => 1024
    };

    let convert_size = |&: size: u64| -> String {
        if matches.opt_present("human-readable") || matches.opt_present("si") {
            if size >= MB {
                format!("{:.1}M", (size as f64) / (MB as f64))
            } else if size >= KB {
                format!("{:.1}K", (size as f64) / (KB as f64))
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
                    show_error!("invalid argument '{}' for 'time style'
Valid arguments are:
- 'full-iso'
- 'long-iso'
- 'iso'
Try '{} --help' for more information.", s, program);
                    return 1;
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
    for path_str in strs.into_iter() {
        let path = Path::new(path_str);
        let stat = safe_unwrap!(fs::lstat(&path));
        let iter = du(&path, Stat{path: path.clone(), fstat: stat}, options_arc.clone(), 0).into_iter();
        let (_, len) = iter.size_hint();
        let len = len.unwrap();
        for (index, stat) in iter.enumerate() {
            let size = match matches.opt_present("apparent-size") {
                true => stat.fstat.unstable.nlink * stat.fstat.size,
                // C's stat is such that each block is assume to be 512 bytes
                // See: http://linux.die.net/man/2/stat
                false => stat.fstat.unstable.blocks * 512,
            };
            if matches.opt_present("time") {
                let tm = {
                    let (secs, nsecs) = {
                        let time = match matches.opt_str("time") {
                            Some(s) => match s.as_slice() {
                                "accessed" => stat.fstat.accessed,
                                "created" => stat.fstat.created,
                                "modified" => stat.fstat.modified,
                                _ => {
                                    show_error!("invalid argument 'modified' for '--time'
    Valid arguments are:
      - 'accessed', 'created', 'modified'
    Try '{} --help' for more information.", program);
                                    return 1;
                                }
                            },
                            None => stat.fstat.modified
                        };
                        ((time / 1000) as i64, (time % 1000 * 1000000) as i32)
                    };
                    time::at(Timespec::new(secs, nsecs))
                };
                if !summarize || (summarize && index == len-1) {
                    let time_str = tm.strftime(time_format_str).unwrap();
                    print!("{}\t{}\t{}{}", convert_size(size), time_str, stat.path.display(), line_separator);
                }
            } else {
                if !summarize || (summarize && index == len-1) {
                    print!("{}\t{}{}", convert_size(size), stat.path.display(), line_separator);
                }
            }
            if options_arc.total && index == (len - 1) {
                // The last element will be the total size of the the path under
                // path_str.  We add it to the grand total.
                grand_total += size;
            }
        }
    }

    if options_arc.total {
        print!("{}\ttotal", convert_size(grand_total));
        print!("{}", line_separator);
    }

    0
}
