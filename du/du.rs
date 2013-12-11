#[link(name="du", vers="1.0.0", author="Derek Chiang")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::stderr;
use std::io::fs;
use std::io::FileStat;
use std::option::Option;
use std::path::Path;
use extra::arc::Arc;
use extra::future::Future;
use extra::getopts::groups;
use extra::time::Timespec;

static VERSION: &'static str = "1.0.0";

struct Options {
    all: bool,
    max_depth: Option<uint>,
    total: bool,
    separate_dirs: bool,
}

fn du(path: &Path, options_arc: Arc<Options>, depth: uint) -> ~[Arc<FileStat>] {
    let mut stats = ~[];
    let mut futures = ~[];
    let options = options_arc.get();
    let mut my_stat = path.stat();

    for f in fs::readdir(path).move_iter() {
        match f.is_file() {
            true => {
                let stat = f.stat();
                my_stat.size += stat.size;
                my_stat.unstable.blocks += stat.unstable.blocks;
                if options.all {
                    stats.push(Arc::new(stat))
                }    
            }
            false => {
                let oa_clone = options_arc.clone();
                futures.push(do Future::spawn { du(&f, oa_clone, depth + 1) })
            }
        }
    }

    for future in futures.mut_iter() {
        for stat_arc in future.get().move_rev_iter() {
            let stat = stat_arc.get();
            if !options.separate_dirs && stat.path.dir_path() == my_stat.path {
                my_stat.size += stat.size;
                my_stat.unstable.blocks += stat.unstable.blocks;
            }
            if options.max_depth == None || depth < options.max_depth.unwrap() {
                stats.push(stat_arc.clone());
            }
        }
    }

    stats.push(Arc::new(my_stat));

    return stats;
}

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        // In task
        groups::optflag("a", "all", " write counts for all files, not just directories"),
        // In main
        groups::optflag("", "apparent-size", "print apparent sizes,  rather  than  disk  usage;
            although  the apparent  size is usually smaller, it may be larger due to holes
            in ('sparse') files, internal  fragmentation,  indirect  blocks, and the like"),
        // In main
        groups::optopt("B", "block-size", "scale sizes  by  SIZE before printing them.
            E.g., '-BM' prints sizes in units of 1,048,576 bytes.  See SIZE format below.",
            "SIZE"),
        // In main
        groups::optflag("b", "bytes", "equivalent to '--apparent-size --block-size=1'"),
        // In main
        groups::optflag("c", "total", "produce a grand total"),
        // In task
        // groups::optflag("D", "dereference-args", "dereference only symlinks that are listed
        //     on the command line"),
        // In main
        // groups::optopt("", "files0-from", "summarize disk usage of the NUL-terminated file
        //                   names specified in file F;
        //                   If F is - then read names from standard input", "F"),
        // // In task
        // groups::optflag("H", "", "equivalent to --dereference-args (-D)"),
        // In main
        groups::optflag("h", "human-readable", "print sizes in human readable format (e.g., 1K 234M 2G)"),
        // In main
        groups::optflag("", "si", "like -h, but use powers of 1000 not 1024"),
        // In main
        groups::optflag("k", "", "like --block-size=1K"),
        // In task
        groups::optflag("l", "count-links", "count sizes many times if hard linked"),
        // // In main
        groups::optflag("m", "", "like --block-size=1M"),
        // // In task
        // groups::optflag("L", "dereference", "dereference all symbolic links"),
        // // In task
        // groups::optflag("P", "no-dereference", "don't follow any symbolic links (this is the default)"),
        // // In main
        groups::optflag("0", "null", "end each output line with 0 byte rather than newline"),
        // In main
        groups::optflag("S", "separate-dirs", "do not include size of subdirectories"),
        // In main
        groups::optflag("s", "summarize", "display only a total for each argument"),
        // // In task
        // groups::optflag("x", "one-file-system", "skip directories on different file systems"),
        // // In task
        // groups::optopt("X", "exclude-from", "exclude files that match any pattern in FILE", "FILE"),
        // // In task
        // groups::optopt("", "exclude", "exclude files that match PATTERN", "PATTERN"),
        // In main
        groups::optopt("d", "max-depth", "print the total for a directory (or file, with --all)
            only if it is N or fewer levels below the command
            line argument;  --max-depth=0 is the same as --summarize", "N"),
        // In main
        groups::optflag("", "time", "show time of the last modification of any file in the
            directory, or any of its subdirectories"),
        // In main
        groups::optopt("", "time", "show time as WORD instead of modification time:
            atime, access, use, ctime or status", "WORD"),
        // In main
        groups::optopt("", "time-style", "show times using style STYLE:
            full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'", "STYLE"),
        groups::optflag("", "help", "display this help and exit"),
        groups::optflag("", "version", "output version information and exit"),
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
        println("du " + VERSION + " - estimate file space usage");
        println("");
        println("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("  {0:s} [OPTION]... --files0-from=F", program);
        println("");
        println(groups::usage("Summarize disk usage of each FILE, recursively for directories.", opts));
        println("Display  values  are  in  units  of  the  first  available  SIZE from
--block-size,  and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environ‐
ment variables.  Otherwise, units default to  1024  bytes  (or  512  if
POSIXLY_CORRECT is set).

SIZE  is  an  integer and optional unit (example: 10M is 10*1024*1024).
Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB, ...  (pow‐
ers of 1000).");
        return
    }

    let options = Options{
        all: matches.opt_present("all"),
        max_depth: match (matches.opt_present("summarize"), matches.opt_str("max-depth")) {
            (true, Some(s)) => match from_str::<uint>(s) {
                Some(_) => {
                    println!("du: warning: summarizing conflicts with --max-depth={:s}", s);
                    return
                },
                None => {
                    println!("du: invalid maximum depth '{:s}'", s);
                    return
                }
            },
            (true, None) => Some(0),
            (false, Some(s)) => match from_str::<uint>(s) {
                Some(u) => Some(u),
                None => {
                    println!("du: invalid maximum depth '{:s}'", s);
                    return
                }
            },
            (false, None) => None
        },
        total: matches.opt_present("total"),
        separate_dirs: matches.opt_present("S"),
    };

    let strs = matches.free.clone();
    let strs = match strs.is_empty() {
        true => ~[~"./"],
        false => strs
    };

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
                    println!("du: invalid --block-size argument '{}'", s);
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
                    println!("du: invalid --block-size argument '{}'", s); return
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
                "full-iso" => "%Y-%m-%d %H:%M:%S.%N %z",
                "long-iso" => "%Y-%m-%d %H:%M",
                "iso" => "%Y-%m-%d",
                _ => {
                    println("
du: invalid argument 'awdwa' for 'time style'
Valid arguments are:
- 'full-iso'
- 'long-iso'
- 'iso'
Try 'du --help' for more information.");
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
    for path_str in strs.iter() {
        let path = Path::new(path_str.clone());
        let iter = du(&path, options_arc.clone(), 0).move_iter();
        let (_, len) = iter.size_hint();
        let len = len.unwrap();
        for (index, stat_arc) in iter.enumerate() {
            let stat = stat_arc.get();
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
                                    println("du: invalid argument 'modified' for '--time'
    Valid arguments are:
      - 'accessed', 'created', 'modified'
    Try 'du --help' for more information.");
                                    return
                                }
                            },
                            None => stat.modified
                        };
                        ((time / 1000) as i64, (time % 1000 * 1000) as i32)
                    };
                    let time_spec = Timespec::new(secs, nsecs);
                    extra::time::at(time_spec).strftime(time_format_str)
                };
                print!("{:<10} {:<30} {}", convert_size(size), time_str, stat.path.display());
            } else {
                print!("{:<10} {}", convert_size(size), stat.path.display());
            }
            print(line_separator);
            if options.total && index == (len - 1) {
                // The last element will be the total size of the the path under
                // path_str.  We add it to the grand total.
                grand_total += size;
            }
        }
    }

    if options.total {
        print!("{:<10} total", convert_size(grand_total));
        print(line_separator);
    }
}
