#![crate_name = "uu_du"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate time;

#[macro_use]
extern crate uucore;

use std::fs;
use std::iter;
use std::io::{stderr, Result, Write};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::collections::HashSet;
use time::Timespec;

const NAME: &'static str = "du";
const SUMMARY: &'static str = "estimate file space usage";
const LONG_HELP: &'static str = "
 Display  values  are  in  units  of  the  first  available  SIZE from
 --block-size,  and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environ‐
 ment variables.  Otherwise, units default to  1024  bytes  (or  512  if
 POSIXLY_CORRECT is set).

 SIZE  is  an  integer and optional unit (example: 10M is 10*1024*1024).
 Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB, ...  (pow‐
 ers of 1000).
";

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
    inode: u64,
    created: u64,
    accessed: u64,
    modified: u64,
}

impl Stat {
    fn new(path: PathBuf) -> Result<Stat> {
        let metadata = fs::symlink_metadata(&path)?;
        Ok(Stat {
            path: path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            blocks: metadata.blocks() as u64,
            nlink: metadata.nlink() as u64,
            inode: metadata.ino() as u64,
            created: metadata.mtime() as u64,
            accessed: metadata.atime() as u64,
            modified: metadata.mtime() as u64,
        })
    }
}

fn get_default_blocks() -> u64 {
    1024
}

// this takes `my_stat` to avoid having to stat files multiple times.
// XXX: this should use the impl Trait return type when it is stabilized
fn du(
    mut my_stat: Stat,
    options: &Options,
    depth: usize,
    inodes: &mut HashSet<u64>,
) -> Box<DoubleEndedIterator<Item = Stat>> {
    let mut stats = vec![];
    let mut futures = vec![];

    if my_stat.is_dir {
        let read = match fs::read_dir(&my_stat.path) {
            Ok(read) => read,
            Err(e) => {
                safe_writeln!(
                    stderr(),
                    "{}: cannot read directory ‘{}‘: {}",
                    options.program_name,
                    my_stat.path.display(),
                    e
                );
                return Box::new(iter::once(my_stat));
            }
        };

        for f in read.into_iter() {
            match f {
                Ok(entry) => match Stat::new(entry.path()) {
                    Ok(this_stat) => {
                        if this_stat.is_dir {
                            futures.push(du(this_stat, options, depth + 1, inodes));
                        } else {
                            if inodes.contains(&this_stat.inode) {
                                continue;
                            }
                            inodes.insert(this_stat.inode);
                            my_stat.size += this_stat.size;
                            my_stat.blocks += this_stat.blocks;
                            if options.all {
                                stats.push(this_stat);
                            }
                        }
                    }
                    Err(error) => show_error!("{}", error),
                },
                Err(error) => show_error!("{}", error),
            }
        }
    }

    stats.extend(
        futures
            .into_iter()
            .flat_map(|val| val)
            .rev()
            .filter_map(|stat| {
                if !options.separate_dirs && stat.path.parent().unwrap() == my_stat.path {
                    my_stat.size += stat.size;
                    my_stat.blocks += stat.blocks;
                }
                if options.max_depth == None || depth < options.max_depth.unwrap() {
                    Some(stat)
                } else {
                    None
                }
            }),
    );
    stats.push(my_stat);
    Box::new(stats.into_iter())
}

pub fn uumain(args: Vec<String>) -> i32 {
    let syntax = format!(
        "[OPTION]... [FILE]...
 {0} [OPTION]... --files0-from=F",
        NAME
    );
    let matches = new_coreopts!(&syntax, SUMMARY, LONG_HELP)
    // In task
        .optflag("a", "all", " write counts for all files, not just directories")
    // In main
        .optflag("", "apparent-size", "print apparent sizes,  rather  than  disk  usage
            although  the apparent  size is usually smaller, it may be larger due to holes
            in ('sparse') files, internal  fragmentation,  indirect  blocks, and the like")
    // In main
        .optopt("B", "block-size", "scale sizes  by  SIZE before printing them.
            E.g., '-BM' prints sizes in units of 1,048,576 bytes.  See SIZE format below.",
            "SIZE")
    // In main
        .optflag("b", "bytes", "equivalent to '--apparent-size --block-size=1'")
    // In main
        .optflag("c", "total", "produce a grand total")
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
        .optflag("h", "human-readable", "print sizes in human readable format (e.g., 1K 234M 2G)")
    // In main
        .optflag("", "si", "like -h, but use powers of 1000 not 1024")
    // In main
        .optflag("k", "", "like --block-size=1K")
    // In task
        .optflag("l", "count-links", "count sizes many times if hard linked")
    // // In main
        .optflag("m", "", "like --block-size=1M")
    // // In task
    // opts.optflag("L", "dereference", "dereference all symbolic links"),
    // // In task
    // opts.optflag("P", "no-dereference", "don't follow any symbolic links (this is the default)"),
    // // In main
        .optflag("0", "null", "end each output line with 0 byte rather than newline")
    // In main
        .optflag("S", "separate-dirs", "do not include size of subdirectories")
    // In main
        .optflag("s", "summarize", "display only a total for each argument")
    // // In task
    // opts.optflag("x", "one-file-system", "skip directories on different file systems"),
    // // In task
    // opts.optopt("X", "exclude-from", "exclude files that match any pattern in FILE", "FILE"),
    // // In task
    // opts.optopt("", "exclude", "exclude files that match PATTERN", "PATTERN"),
    // In main
        .optopt("d", "max-depth", "print the total for a directory (or file, with --all)
            only if it is N or fewer levels below the command
            line argument;  --max-depth=0 is the same as --summarize", "N")
    // In main
        .optflagopt("", "time", "show time of the last modification of any file in the
            directory, or any of its subdirectories.  If WORD is given, show time as WORD instead
            of modification time: atime, access, use, ctime or status", "WORD")
    // In main
        .optopt("", "time-style", "show times using style STYLE:
            full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'", "STYLE")
        .parse(args);

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
        program_name: NAME.to_owned(),
        max_depth: max_depth,
        total: matches.opt_present("total"),
        separate_dirs: matches.opt_present("S"),
    };

    let strs = if matches.free.is_empty() {
        vec!["./".to_owned()]
    } else {
        matches.free.clone()
    };

    let mb = if matches.opt_present("si") {
        1000 * 1000
    } else {
        1024 * 1024
    };
    let kb = if matches.opt_present("si") {
        1000
    } else {
        1024
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
        None => get_default_blocks(),
    };

    let convert_size = |size: u64| -> String {
        if matches.opt_present("human-readable") || matches.opt_present("si") {
            if size >= mb {
                format!("{:.1}M", (size as f64) / (mb as f64))
            } else if size >= kb {
                format!("{:.1}K", (size as f64) / (kb as f64))
            } else {
                format!("{}B", size)
            }
        } else if matches.opt_present("k") {
            format!("{}", ((size as f64) / (kb as f64)).ceil())
        } else if matches.opt_present("m") {
            format!("{}", ((size as f64) / (mb as f64)).ceil())
        } else {
            format!("{}", ((size as f64) / (block_size as f64)).ceil())
        }
    };

    let time_format_str = match matches.opt_str("time-style") {
        Some(s) => match &s[..] {
            "full-iso" => "%Y-%m-%d %H:%M:%S.%f %z",
            "long-iso" => "%Y-%m-%d %H:%M",
            "iso" => "%Y-%m-%d",
            _ => {
                show_error!(
                    "invalid argument '{}' for 'time style'
Valid arguments are:
- 'full-iso'
- 'long-iso'
- 'iso'
Try '{} --help' for more information.",
                    s,
                    NAME
                );
                return 1;
            }
        },
        None => "%Y-%m-%d %H:%M",
    };

    let line_separator = if matches.opt_present("0") { "\0" } else { "\n" };

    let mut grand_total = 0;
    for path_str in strs.into_iter() {
        let path = PathBuf::from(&path_str);
        match Stat::new(path) {
            Ok(stat) => {
                let mut inodes: HashSet<u64> = HashSet::new();

                let iter = du(stat, &options, 0, &mut inodes).into_iter();
                let (_, len) = iter.size_hint();
                let len = len.unwrap();
                for (index, stat) in iter.enumerate() {
                    let size = if matches.opt_present("apparent-size") {
                        stat.nlink * stat.size
                    } else {
                        // C's stat is such that each block is assume to be 512 bytes
                        // See: http://linux.die.net/man/2/stat
                        stat.blocks * 512
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
                                            show_error!(
                                                "invalid argument 'modified' for '--time'
    Valid arguments are:
      - 'accessed', 'created', 'modified'
    Try '{} --help' for more information.",
                                                NAME
                                            );
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
                            print!(
                                "{}\t{}\t{}{}",
                                convert_size(size),
                                time_str,
                                stat.path.display(),
                                line_separator
                            );
                        }
                    } else {
                        if !summarize || (summarize && index == len - 1) {
                            print!(
                                "{}\t{}{}",
                                convert_size(size),
                                stat.path.display(),
                                line_separator
                            );
                        }
                    }
                    if options.total && index == (len - 1) {
                        // The last element will be the total size of the the path under
                        // path_str.  We add it to the grand total.
                        grand_total += size;
                    }
                }
            }
            Err(_) => {
                show_error!("{}: {}", path_str, "No such file or directory");
            }
        }
    }

    if options.total {
        print!("{}\ttotal", convert_size(grand_total));
        print!("{}", line_separator);
    }

    0
}
