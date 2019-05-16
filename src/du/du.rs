#![crate_name = "uu_du"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 * (c) Alex Lyon <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate structopt;

#[macro_use]
extern crate uucore;

use std::env;
use std::path::PathBuf;
use structopt::{StructOpt, clap::{AppSettings, arg_enum}};

use walker::DuWalker;

mod walker;

const NAME: &str = "du";
const SUMMARY: &str = "estimate file space usage";
const LONG_HELP: &str = "
 Display  values  are  in  units  of  the  first  available  SIZE from
 --block-size,  and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environ‐
 ment variables.  Otherwise, units default to  1024  bytes  (or  512  if
 POSIXLY_CORRECT is set).

 SIZE  is  an  integer and optional unit (example: 10M is 10*1024*1024).
 Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB, ...  (pow‐
 ers of 1000).
";

const UNITS: [(char, u32); 8] = [('Y', 8), ('Z', 7), ('E', 6), ('P', 5), ('T', 4), ('G', 3), ('M', 2), ('K', 1)];

arg_enum! {
    #[derive(Debug, Clone, Copy)]
    enum TimeStrategy {
        Atime,
        Access,
        Use,
        Ctime,
        Status,
    }
}

arg_enum! {
    #[derive(Debug, Clone, Copy)]
    enum TimeFormat {
        FullIso,
        LongIso,
        Iso,
    }
}

#[derive(StructOpt, Debug)]
struct SizeFormat {
    /// Print sizes in human readable format (e.g., 1K 234M 2G)
    #[structopt(short = "h", long = "human-readable")]
    human_readable: bool,

    /// Like -h, but use powers of 1000 not 1024
    #[structopt(long = "si", raw(overrides_with = r#""human-readable""#))]
    si: bool,
}

#[derive(StructOpt, Debug)]
struct BlockSize {
    /// Scale sizes by SIZE before printing them.  For example, '-BM' prints sizes in units of
    /// 1,048,576 bytes.  See SIZE format below
    #[structopt(short = "B", long = "block-size", parse(try_from_str = "unit_string_to_number"))]
    block_size: Option<u128>,

    /// Like --block-size=1K
    #[structopt(short = "k", overrides_with = r#""mega""#)]
    kilo: bool,

    /// Like --block-size=1M
    #[structopt(short = "m", overrides_with = r#""kilo""#)]
    mega: bool,
}

// FIXME: check for all needed overrides/conflicts
#[derive(StructOpt, Debug)]
#[structopt(raw(name = "NAME", setting = "AppSettings::AllArgsOverrideSelf"))]
struct Options {
    /// Write counts for all files, not just directories
    #[structopt(short = "a", long = "all", conflicts_with = "summarize")]
    all: bool,

    /// Print apparent sizes, rather than disk usage.  Although the apparent size is usually
    /// smaller, it may be larger due to holes in ('sparse') files, internal fragmentation,
    /// indirect blocks, and the like
    #[structopt(long = "apparent-size")]
    apparent_size: bool,

    #[structopt(flatten)]
    block_size: BlockSize,

    #[structopt(flatten)]
    size_format: SizeFormat,

    // XXX: it would be nice if there's a way to set this automatically
    /// Equivalent to '--apparent-size --block-size=1'
    #[structopt(short = "b", long = "bytes", raw(overrides_with = r#""block_size""#))]
    bytes: bool,

    /// Produce a grand total
    #[structopt(short = "c", long = "total")]
    total: bool,

    /// Count sizes many times if hard-linked
    #[structopt(short = "l", long = "count-links")]
    count_links: bool,

    /// End each output line with 0 byte rather than newline
    #[structopt(short = "0", long = "null")]
    null: bool,

    /// Do not include the size of subdirectories
    #[structopt(short = "S", long = "separate-dirs")]
    separate_dirs: bool,

    /// Display only a total for each argument
    #[structopt(short = "s", long = "summarize", conflicts_with = "max_depth")]
    summarize: bool,

    /// Print the total for a directory (or file, with --all) only if it is N or fewer levels below
    /// the command line argument; --max-depth=0 is the same as --summarize
    #[structopt(short = "d", long = "max-depth", parse(try_from_str))]
    max_depth: Option<usize>,

    // FIXME: this is broken, needs to handle case where no option is given (i.e. just "--time" is given), which
    //        should make the time strategy use mtime instead of atime/ctime
    /// Show time of the last modification of any file in the directory, or any of its
    /// subdirectories.  If WORD is given, show time as WORD instead of modification time: atime,
    /// access, use, ctime or status
    #[structopt(long = "time", raw(possible_values = "&TimeStrategy::variants()"), case_insensitive = true)]
    time: Option<TimeStrategy>,

    // FIXME: need to implement +FORMAT
    /// Show times using style STYLE: full-iso, long-iso, iso, +FORMAT (where FORMAT is interpreted
    /// like 'date'; the +FORMAT functionality is NOT IMPLEMENTED)
    #[structopt(long = "time-style", min_values = 0, raw(possible_values = "&TimeFormat::variants()"), case_insensitive = true)]
    time_style: Option<TimeFormat>,

    /// Search through directories using COUNT threads.  If COUNT is 0, the default number of
    /// threads will be used
    #[structopt(long = "thread-count", default_value = "0", parse(try_from_str))]
    thread_count: usize,

    #[structopt(multiple = true, parse(from_os_str))]
    files: Vec<PathBuf>,
}

    // In task
    // opts.optflag("D", "dereference-args", "dereference only symlinks that are listed
    //     on the command line"),
    // In main
    // opts.optopt("", "files0-from", "summarize disk usage of the NUL-terminated file
    //                   names specified in file F;
    //                   If F is - then read names from standard input", "F"),
    // // In task
    // opts.optflag("H", "", "equivalent to --dereference-args (-D)"),
    // // In task
    // opts.optflag("L", "dereference", "dereference all symbolic links"),
    // // In task
    // opts.optflag("P", "no-dereference", "don't follow any symbolic links (this is the default)"),
    // // In task
    // opts.optflag("x", "one-file-system", "skip directories on different file systems"),
    // // In task
    // opts.optopt("X", "exclude-from", "exclude files that match any pattern in FILE", "FILE"),
    // // In task
    // opts.optopt("", "exclude", "exclude files that match PATTERN", "PATTERN"),

fn retrieve_default_block_size() -> u128 {
    for env_var in &["DU_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"] {
        if let Some(quantity) = translate_to_pure_number(&env::var(env_var).ok()) {
            return quantity;
        }
    }

    if env::var("POSIXLY_CORRECT").is_ok() {
        512
    } else {
        1024
    }
}

fn split_number_and_scale(s: &str, idx: usize) -> Option<(&str, u128)> {
    let (number_s, suffix) = s.split_at(idx);

    let first_ch = suffix.chars().next()?.to_ascii_uppercase();

    // search for the unit (e.g. the "K" in "KiB") in the given suffix
    let unit = UNITS
        .iter()
        .rev()
        .find_map(|&(unit_ch, val)| {
            if unit_ch == first_ch {
                Some(val)
            } else {
                None
            }
        });

    let scale = if let Some(unit) = unit {
        // check for multiplier modifiers
        let multiple: u128 = match &suffix[1..] {
            "iB" | "" => 1024,
            "B" | "b" => 1000,
            _ => return None,
        };
        multiple.pow(unit)
    } else if first_ch == 'B' && suffix.len() == 1 {
        1000
    } else {
        return None
    };

    Some((number_s, scale))
}

fn unit_string_to_number(s: &str) -> Result<u128, String> {
    let (number_s, scale) = match s.rfind(|ch: char| ch.is_digit(10)) {
        Some(idx) if idx + 1 < s.len() => split_number_and_scale(s, idx + 1)
            .ok_or_else(|| format!("invalid suffix: {}", &s[idx + 1..]))?,
        _ => (s, 1),
    };

    let number = number_s.parse::<u128>().map_err(|e| format!("{}", e))?;

    Ok(number * scale)
}

fn translate_to_pure_number(s: &Option<String>) -> Option<u128> {
    s.as_ref().and_then(|s| unit_string_to_number(&s).ok())
}

fn convert_size_human(size: u64, multiplier: u64, _block_size: u128) -> String {
    for &(unit, power) in &UNITS {
        let limit = multiplier.pow(power);
        if size >= limit {
            return format!("{:.1}{}", (size as f64) / (limit as f64), unit);
        }
    }
    format!("{}B", size)
}

fn convert_size_b(size: u64, _multiplier: u64, _block_size: u128) -> String {
    format!("{}", ((size as f64) / (1 as f64)).ceil())
}

fn convert_size_k(size: u64, multiplier: u64, _block_size: u128) -> String {
    format!("{}", ((size as f64) / (multiplier as f64)).ceil())
}

fn convert_size_m(size: u64, multiplier: u64, _block_size: u128) -> String {
    format!("{}", ((size as f64) / ((multiplier * multiplier) as f64)).ceil())
}

fn convert_size_other(size: u64, _multiplier: u64, block_size: u128) -> String {
    format!("{}", ((size as f64) / (block_size as f64)).ceil())
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut options = Options::from_iter(args);

    if options.files.is_empty() {
        // FIXME: does this print correctly if we drop the trailing slash?
        options.files.push(PathBuf::from("./"));
    }

    let block_size = options.block_size.block_size.unwrap_or_else(retrieve_default_block_size);

    let multiplier: u64 = if options.size_format.si {
        1000
    } else {
        1024
    };
    let convert_size_fn = {
        if options.size_format.human_readable || options.size_format.si {
            convert_size_human        
        } else if options.bytes {
            convert_size_b
        } else if options.block_size.kilo {
            convert_size_k
        } else if options.block_size.mega {
            convert_size_m
        } else {
            convert_size_other
        }
    };
    let convert_size = |size| convert_size_fn(size, multiplier, block_size);

    let line_separator = if options.null { "\0" } else { "\n" };

    let mut grand_total = 0;
    for path in &options.files {
        // FIXME: obviously needs cleanup, as does all the command-line parsing stuff
        // FIXME: figure out how not to clone this

        // FIXME!!!!: this does not work if the given path is a file, not a directory
        let walker = match DuWalker::new(path.clone(), &options, convert_size) {
            Ok(walker) => walker,
            // XXX: should we actually show the error?
            Err(_) => {
                show_error!("{}: {}", path.display(), "No such file or directory");
                continue;
            }
        };

        if let Some(size) = walker.run() {
            if options.total {
                grand_total += size;
            }
        }
    }

    if options.total {
        print!("{}\ttotal", convert_size(grand_total));
        print!("{}", line_separator);
    }

    0
}

#[cfg(test)]
mod test_du {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_translate_to_pure_number() {
        let test_data = [
            (Some("10".to_string()), Some(10)),
            (Some("10K".to_string()), Some(10 * 1024)),
            (Some("5M".to_string()), Some(5 * 1024 * 1024)),
            (Some("900KB".to_string()), Some(900 * 1000)),
            (Some("BAD_STRING".to_string()), None),
        ];
        for it in test_data.into_iter() {
            assert_eq!(translate_to_pure_number(&it.0), it.1);
        }
    }

    #[test]
    fn test_read_block_size() {
        let test_data = [
            (Some("10".to_string()), 10),
            (None, 1024),
            (Some("BAD_STRING".to_string()), 1024),
        ];
        for it in test_data.into_iter() {
            assert_eq!(read_block_size(it.0.clone()), it.1);
        }
    }
}
