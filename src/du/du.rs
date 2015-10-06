#![crate_name = "du"]

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

use std::fs;
use std::io::{stderr, Write};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::Arc;
use time::Timespec;
use std::sync::mpsc::channel;
use std::thread;

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
    path: PathBuf,
    is_dir: bool,
    size: u64,
    blocks: u64,
    nlink: u64,
    created: u64,
    accessed: u64,
    modified: u64,
}

impl Stat {
    fn new(path: &PathBuf) -> Stat {
        let metadata = safe_unwrap!(fs::metadata(path));
        Stat {
            path: path.clone(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            blocks: metadata.blocks() as u64,
            nlink: metadata.nlink() as u64,
            created: metadata.mtime() as u64,
            accessed: metadata.atime() as u64,
            modified: metadata.mtime() as u64,
        }
    }
}

// this takes `my_stat` to avoid having to stat files multiple times.
fn du(path: &PathBuf, mut my_stat: Stat, options: Arc<Options>, depth: usize) -> Vec<Arc<Stat>> {
    let mut stats = vec!();
    let mut futures = vec!();

    if my_stat.is_dir {
        let read = match fs::read_dir(path) {
            Ok(read) => read,
            Err(e) => {
                safe_writeln!(stderr(),
                              "{}: cannot read directory ‘{}‘: {}",
                              options.program_name,
                              path.display(),
                              e);
                return vec!(Arc::new(my_stat))
            }
        };

        for f in read.into_iter() {
            let entry = f.unwrap();
            let this_stat = Stat::new(&entry.path());
            if this_stat.is_dir {
                let oa_clone = options.clone();
                let (tx, rx) = channel();
                thread::spawn(move || {
                    let result = du(&entry.path(), this_stat, oa_clone, depth + 1);
                    tx.send(result)
                });
                futures.push(rx);
            } else {
                my_stat.size += this_stat.size;
                my_stat.blocks += this_stat.blocks;
                if options.all {
                    stats.push(Arc::new(this_stat))
                }
            }
        }
    }

    for rx in futures.iter_mut() {
        for stat in rx.recv().unwrap().into_iter().rev() {
            if !options.separate_dirs && stat.path.parent().unwrap().to_path_buf() == my_stat.path {
                my_stat.size += stat.size;
                my_stat.blocks += stat.blocks;
            }
            if options.max_depth == None || depth < options.max_depth.unwrap() {
                stats.push(stat.clone());
            }
        }
    }

    stats.push(Arc::new(my_stat));

    stats
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    // In task
    opts.optflag("a",
                 "all",
                 " write counts for all files, not just directories");
    // In main
    opts.optflag("",
                 "apparent-size",
                 "print apparent sizes,  rather  than  disk  usage;
            although  the apparent  size is usually smaller, it may be larger due to holes
            in ('sparse') files, internal  fragmentation,  indirect  blocks, and the like");
    // In main
    opts.optopt("B",
                "block-size",
                "scale sizes  by  SIZE before printing them.
            E.g., '-BM' prints sizes in units of 1,048,576 bytes.  See SIZE format below.",
                "SIZE");
    // In main
    opts.optflag("b",
                 "bytes",
                 "equivalent to '--apparent-size --block-size=1'");
    // In main
    opts.optflag("c", "total", "produce a grand total");
    // In task
    // opts.optflag("D", "dereference-args", "dereference only symlinks that are listed
    //     on the command line"),
    // In main
    // opts.optopt("", "files0-from", "summarize disk usage of the NUL-terminated file
    //                   names specified in file F;
    //                   If F is - then read names from standard input", "F"),
    // // In task
    // opts.optflag("H", "", "equivalent to --dereference-args (-D)"),
    // In main
    opts.optflag("h",
                 "human-readable",
                 "print sizes in human readable format (e.g., 1K 234M 2G)");
    // In main
    opts.optflag("", "si", "like -h, but use powers of 1000 not 1024");
    // In main
    opts.optflag("k", "", "like --block-size=1K");
    // In task
    opts.optflag("l",
                 "count-links",
                 "count sizes many times if hard linked");
    // // In main
    opts.optflag("m", "", "like --block-size=1M");
    // // In task
    // opts.optflag("L", "dereference", "dereference all symbolic links"),
    // // In task
    // opts.optflag("P", "no-dereference", "don't follow any symbolic links (this is the default)"),
    // // In main
    opts.optflag("0",
                 "null",
                 "end each output line with 0 byte rather than newline");
    // In main
    opts.optflag("S",
                 "separate-dirs",
                 "do not include size of subdirectories");
    // In main
    opts.optflag("s",
                 "summarize",
                 "display only a total for each argument");
    // // In task
    // opts.optflag("x", "one-file-system", "skip directories on different file systems"),
    // // In task
    // opts.optopt("X", "exclude-from", "exclude files that match any pattern in FILE", "FILE"),
    // // In task
    // opts.optopt("", "exclude", "exclude files that match PATTERN", "PATTERN"),
    // In main
    opts.optopt("d",
                "max-depth",
                "print the total for a directory (or file, with --all)
            only if it is N or fewer levels below the command
            line argument;  --max-depth=0 is the same as --summarize",
                "N");
    // In main
    opts.optflagopt("", "time", "show time of the last modification of any file in the
            directory, or any of its subdirectories.  If WORD is given, show time as WORD instead of modification time:
            atime, access, use, ctime or status", "WORD");
    // In main
    opts.optopt("",
                "time-style",
                "show times using style STYLE:
            full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'",
                "STYLE");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
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
                 program = NAME,
                 version = VERSION,
                 usage =
                     opts.usage("Summarize disk usage of each FILE, recursively for directories."));
        return 0;
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
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
        (Some(_), Some(_)) | (None, _) => { /* valid */
        }
    }

    let options = Options {
        all: matches.opt_present("all"),
        program_name: NAME.to_string(),
        max_depth: max_depth,
        total: matches.opt_present("total"),
        separate_dirs: matches.opt_present("S"),
    };

    let strs = if matches.free.is_empty() {
        vec!("./".to_string())
    } else {
        matches.free.clone()
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
            let mut numbers = String::new();
            let mut letters = String::new();
            for c in s.chars() {
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
            let number = numbers.parse::<u64>().unwrap();
            let multiple = match &letters[..] {
                "K" => 1024u64.pow(1),
                "M" => 1024u64.pow(2),
                "G" => 1024u64.pow(3),
                "T" => 1024u64.pow(4),
                "P" => 1024u64.pow(5),
                "E" => 1024u64.pow(6),
                "Z" => 1024u64.pow(7),
                "Y" => 1024u64.pow(8),
                "KB" => 1000u64.pow(1),
                "MB" => 1000u64.pow(2),
                "GB" => 1000u64.pow(3),
                "TB" => 1000u64.pow(4),
                "PB" => 1000u64.pow(5),
                "EB" => 1000u64.pow(6),
                "ZB" => 1000u64.pow(7),
                "YB" => 1000u64.pow(8),
                _ => {
                    show_error!("invalid --block-size argument '{}'", s);
                    return 1;
                }
            };
            number * multiple
        }
        None => 1024,
    };

    let convert_size = |size: u64| -> String {
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
            match &s[..] {
                "full-iso" => "%Y-%m-%d %H:%M:%S.%f %z",
                "long-iso" => "%Y-%m-%d %H:%M",
                "iso" => "%Y-%m-%d",
                _ => {
                    show_error!("invalid argument '{}' for 'time style'
Valid arguments are:
- 'full-iso'
- 'long-iso'
- 'iso'
Try '{} --help' for more information.",
                                s,
                                NAME);
                    return 1;
                }
            }
        }
        None => "%Y-%m-%d %H:%M",
    };

    let line_separator = match matches.opt_present("0") {
        true => "\0",
        false => "\n",
    };

    let mut grand_total = 0;
    for path_str in strs.into_iter() {
        let path = PathBuf::from(path_str);
        let iter = du(&path, Stat::new(&path), options_arc.clone(), 0).into_iter();
        let (_, len) = iter.size_hint();
        let len = len.unwrap();
        for (index, stat) in iter.enumerate() {
            let size = match matches.opt_present("apparent-size") {
                true => stat.nlink * stat.size,
                // C's stat is such that each block is assume to be 512 bytes
                // See: http://linux.die.net/man/2/stat
                false => stat.blocks * 512,
            };
            if matches.opt_present("time") {
                let tm = {
                    let (secs, nsecs) = {
                        let time = match matches.opt_str("time") {
                            Some(s) => match &s[..] {
                                "accessed" => stat.accessed,
                                "created" => stat.created,
                                "modified" => stat.modified,
                                _ => {
                                    show_error!("invalid argument 'modified' for '--time'
    Valid arguments are:
      - 'accessed', 'created', 'modified'
    Try '{} --help' for more information.",
                                                NAME);
                                    return 1;
                                }
                            },
                            None => stat.modified,
                        };
                        ((time / 1000) as i64, (time % 1000 * 1000000) as i32)
                    };
                    time::at(Timespec::new(secs, nsecs))
                };
                if !summarize || (summarize && index == len - 1) {
                    let time_str = tm.strftime(time_format_str).unwrap();
                    print!("{}\t{}\t{}{}",
                           convert_size(size),
                           time_str,
                           stat.path.display(),
                           line_separator);
                }
            } else {
                if !summarize || (summarize && index == len - 1) {
                    print!("{}\t{}{}",
                           convert_size(size),
                           stat.path.display(),
                           line_separator);
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
