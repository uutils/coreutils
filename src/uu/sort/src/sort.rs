//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Yin <mikeyin@mikeyin.org>
//  * (c) Robert Swinford <robert.swinford..AT..gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
#![allow(dead_code)]

// Although they don't always seem to decribe reality, check out the POSIX and GNU specs:
// https://pubs.opengroup.org/onlinepubs/9699919799/utilities/sort.html
// https://www.gnu.org/software/coreutils/manual/html_node/sort-invocation.html

// spell-checker:ignore (ToDO) outfile nondictionary
#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use fnv::FnvHasher;
use itertools::Itertools;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use semver::Version;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Lines, Read, Write};
use std::mem::replace;
use std::path::Path;
use uucore::fs::is_stdin_interactive; // for Iterator::dedup()

static NAME: &str = "sort";
static ABOUT: &str = "Display sorted concatenation of all FILE(s).";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_HUMAN_NUMERIC_SORT: &str = "human-numeric-sort";
static OPT_MONTH_SORT: &str = "month-sort";
static OPT_NUMERIC_SORT: &str = "numeric-sort";
static OPT_GENERAL_NUMERIC_SORT: &str = "general-numeric-sort";
static OPT_VERSION_SORT: &str = "version-sort";

static OPT_DICTIONARY_ORDER: &str = "dictionary-order";
static OPT_MERGE: &str = "merge";
static OPT_CHECK: &str = "check";
static OPT_CHECK_SILENT: &str = "check-silent";
static OPT_IGNORE_CASE: &str = "ignore-case";
static OPT_IGNORE_BLANKS: &str = "ignore-blanks";
static OPT_IGNORE_NONPRINTING: &str = "ignore-nonprinting";
static OPT_OUTPUT: &str = "output";
static OPT_REVERSE: &str = "reverse";
static OPT_STABLE: &str = "stable";
static OPT_UNIQUE: &str = "unique";
static OPT_RANDOM: &str = "random-sort";
static OPT_ZERO_TERMINATED: &str = "zero-terminated";

static ARG_FILES: &str = "files";

static DECIMAL_PT: char = '.';
static THOUSANDS_SEP: char = ',';
static NEGATIVE: char = '-';
static POSITIVE: char = '+';
static ENOTATION: char = 'E';

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum SortMode {
    Numeric,
    HumanNumeric,
    GeneralNumeric,
    Month,
    Version,
    Default,
}

struct Settings {
    mode: SortMode,
    merge: bool,
    reverse: bool,
    outfile: Option<String>,
    stable: bool,
    unique: bool,
    check: bool,
    check_silent: bool,
    random: bool,
    compare_fns: Vec<fn(&str, &str, &Settings) -> Ordering>,
    transform_fns: Vec<fn(&str) -> String>,
    salt: String,
    zero_terminated: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            mode: SortMode::Default,
            merge: false,
            reverse: false,
            outfile: None,
            stable: false,
            unique: false,
            check: false,
            check_silent: false,
            random: false,
            compare_fns: Vec::new(),
            transform_fns: Vec::new(),
            salt: String::new(),
            zero_terminated: false,
        }
    }
}

struct MergeableFile<'a> {
    lines: Lines<BufReader<Box<dyn Read>>>,
    current_line: String,
    settings: &'a Settings,
}

// BinaryHeap depends on `Ord`. Note that we want to pop smallest items
// from the heap first, and BinaryHeap.pop() returns the largest, so we
// trick it into the right order by calling reverse() here.
impl<'a> Ord for MergeableFile<'a> {
    fn cmp(&self, other: &MergeableFile) -> Ordering {
        compare_by(&self.current_line, &other.current_line, &self.settings).reverse()
    }
}

impl<'a> PartialOrd for MergeableFile<'a> {
    fn partial_cmp(&self, other: &MergeableFile) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialEq for MergeableFile<'a> {
    fn eq(&self, other: &MergeableFile) -> bool {
        Ordering::Equal == compare_by(&self.current_line, &other.current_line, &self.settings)
    }
}

impl<'a> Eq for MergeableFile<'a> {}

struct FileMerger<'a> {
    heap: BinaryHeap<MergeableFile<'a>>,
    settings: &'a Settings,
}

impl<'a> FileMerger<'a> {
    fn new(settings: &'a Settings) -> FileMerger<'a> {
        FileMerger {
            heap: BinaryHeap::new(),
            settings,
        }
    }
    fn push_file(&mut self, mut lines: Lines<BufReader<Box<dyn Read>>>) {
        if let Some(Ok(next_line)) = lines.next() {
            let mergeable_file = MergeableFile {
                lines,
                current_line: next_line,
                settings: &self.settings,
            };
            self.heap.push(mergeable_file);
        }
    }
}

impl<'a> Iterator for FileMerger<'a> {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        match self.heap.pop() {
            Some(mut current) => {
                match current.lines.next() {
                    Some(Ok(next_line)) => {
                        let ret = replace(&mut current.current_line, next_line);
                        self.heap.push(current);
                        Some(ret)
                    }
                    _ => {
                        // Don't put it back in the heap (it's empty/erroring)
                        // but its first line is still valid.
                        Some(current.current_line)
                    }
                }
            }
            None => None,
        }
    }
}

fn get_usage() -> String {
    format!(
        "{0} {1}
Usage:
 {0} [OPTION]... [FILE]...
Write the sorted concatenation of all FILE(s) to standard output.
Mandatory arguments for long options are mandatory for short options too.
With no FILE, or when FILE is -, read standard input.",
        NAME, VERSION
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();
    let usage = get_usage();
    let mut settings: Settings = Default::default();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_HUMAN_NUMERIC_SORT)
                .short("h")
                .long(OPT_HUMAN_NUMERIC_SORT)
                .help("compare according to human readable sizes, eg 1M > 100k"),
        )
        .arg(
            Arg::with_name(OPT_MONTH_SORT)
                .short("M")
                .long(OPT_MONTH_SORT)
                .help("compare according to month name abbreviation"),
        )
        .arg(
            Arg::with_name(OPT_NUMERIC_SORT)
                .short("n")
                .long(OPT_NUMERIC_SORT)
                .help("compare according to string numerical value"),
        )
        .arg(
            Arg::with_name(OPT_GENERAL_NUMERIC_SORT)
                .short("g")
                .long(OPT_GENERAL_NUMERIC_SORT)
                .help("compare according to string general numerical value"),
        )
        .arg(
            Arg::with_name(OPT_VERSION_SORT)
                .short("V")
                .long(OPT_VERSION_SORT)
                .help("Sort by SemVer version number, eg 1.12.2 > 1.1.2"),
        )
        .arg(
            Arg::with_name(OPT_DICTIONARY_ORDER)
                .short("d")
                .long(OPT_DICTIONARY_ORDER)
                .help("consider only blanks and alphanumeric characters"),
        )
        .arg(
            Arg::with_name(OPT_MERGE)
                .short("m")
                .long(OPT_MERGE)
                .help("merge already sorted files; do not sort"),
        )
        .arg(
            Arg::with_name(OPT_CHECK)
                .short("c")
                .long(OPT_CHECK)
                .help("check for sorted input; do not sort"),
        )
        .arg(
            Arg::with_name(OPT_CHECK_SILENT)
                .short("C")
                .long(OPT_CHECK_SILENT)
                .help("exit successfully if the given file is already sorted, and exit with status 1 otherwise. "),
        )
        .arg(
            Arg::with_name(OPT_IGNORE_CASE)
                .short("f")
                .long(OPT_IGNORE_CASE)
                .help("fold lower case to upper case characters"),
        )
        .arg(
            Arg::with_name(OPT_IGNORE_NONPRINTING)
                .short("-i")
                .long(OPT_IGNORE_NONPRINTING)
                .help("ignore nonprinting characters"),
        )
        .arg(
            Arg::with_name(OPT_IGNORE_BLANKS)
                .short("b")
                .long(OPT_IGNORE_BLANKS)
                .help("ignore leading blanks when finding sort keys in each line"),
        )
        .arg(
            Arg::with_name(OPT_OUTPUT)
                .short("o")
                .long(OPT_OUTPUT)
                .help("write output to FILENAME instead of stdout")
                .takes_value(true)
                .value_name("FILENAME"),
        )
        .arg(
            Arg::with_name(OPT_RANDOM)
                .short("R")
                .long(OPT_RANDOM)
                .help("shuffle in random order"),
        )
        .arg(
            Arg::with_name(OPT_REVERSE)
                .short("r")
                .long(OPT_REVERSE)
                .help("reverse the output"),
        )
        .arg(
            Arg::with_name(OPT_STABLE)
                .short("s")
                .long(OPT_STABLE)
                .help("stabilize sort by disabling last-resort comparison"),
        )
        .arg(
            Arg::with_name(OPT_UNIQUE)
                .short("u")
                .long(OPT_UNIQUE)
                .help("output only the first of an equal run"),
        )
        .arg(
            Arg::with_name(OPT_ZERO_TERMINATED)
                .short("z")
                .long(OPT_ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
        .get_matches_from(args);

    let mut files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    settings.mode = if matches.is_present(OPT_HUMAN_NUMERIC_SORT) {
        SortMode::HumanNumeric
    } else if matches.is_present(OPT_MONTH_SORT) {
        SortMode::Month
    } else if matches.is_present(OPT_GENERAL_NUMERIC_SORT) {
        SortMode::GeneralNumeric
    } else if matches.is_present(OPT_NUMERIC_SORT) {
        SortMode::Numeric
    } else if matches.is_present(OPT_VERSION_SORT) {
        SortMode::Version
    } else {
        SortMode::Default
    };

    if matches.is_present(OPT_DICTIONARY_ORDER) {
        settings.transform_fns.push(remove_nondictionary_chars);
    } else if matches.is_present(OPT_IGNORE_NONPRINTING) {
        settings.transform_fns.push(remove_nonprinting_chars);
    }

    settings.zero_terminated = matches.is_present(OPT_ZERO_TERMINATED);
    settings.merge = matches.is_present(OPT_MERGE);

    settings.check = matches.is_present(OPT_CHECK);
    if matches.is_present(OPT_CHECK_SILENT) {
        settings.check_silent = matches.is_present(OPT_CHECK_SILENT);
        settings.check = true;
    };

    if matches.is_present(OPT_IGNORE_CASE) {
        settings.transform_fns.push(|s| s.to_uppercase());
    }

    if matches.is_present(OPT_IGNORE_BLANKS) {
        settings.transform_fns.push(|s| s.trim_start().to_string());
    }

    settings.outfile = matches.value_of(OPT_OUTPUT).map(String::from);
    settings.reverse = matches.is_present(OPT_REVERSE);
    settings.stable = matches.is_present(OPT_STABLE);
    settings.unique = matches.is_present(OPT_UNIQUE);

    if matches.is_present(OPT_RANDOM) {
        settings.random = matches.is_present(OPT_RANDOM);
        settings.salt = get_rand_string();
    }

    //let mut files = matches.free;
    if files.is_empty() {
        /* if no file, default to stdin */
        files.push("-".to_owned());
    } else if settings.check && files.len() != 1 {
        crash!(1, "sort: extra operand `{}' not allowed with -c", files[1])
    }

    settings.compare_fns.push(match settings.mode {
        SortMode::Numeric => numeric_compare,
        SortMode::GeneralNumeric => general_numeric_compare,
        SortMode::HumanNumeric => human_numeric_size_compare,
        SortMode::Month => month_compare,
        SortMode::Version => version_compare,
        SortMode::Default => default_compare,
    });

    if !settings.stable {
        match settings.mode {
            SortMode::Default => {}
            _ => settings.compare_fns.push(default_compare),
        }
    }

    exec(files, &mut settings)
}

fn exec(files: Vec<String>, settings: &mut Settings) -> i32 {
    let mut lines = Vec::new();
    let mut file_merger = FileMerger::new(&settings);

    for path in &files {
        let (reader, _) = match open(path) {
            Some(x) => x,
            None => continue,
        };

        let buf_reader = BufReader::new(reader);

        if settings.merge {
            file_merger.push_file(buf_reader.lines());
        } else if settings.check {
            return exec_check_file(buf_reader.lines(), &settings);
        } else if settings.zero_terminated {
            for line in buf_reader.split(b'\0') {
                if let Ok(n) = line {
                    lines.push(std::str::from_utf8(&n).unwrap_or("\0").to_string());
                } else {
                    break;
                }
            }
        } else {
            for line in buf_reader.lines() {
                if let Ok(n) = line {
                    lines.push(n);
                } else {
                    break;
                }
            }
        }
    }

    sort_by(&mut lines, &settings);

    if settings.merge {
        if settings.unique {
            print_sorted(file_merger.dedup(), &settings)
        } else {
            print_sorted(file_merger, &settings)
        }
    } else if settings.mode == SortMode::Month && settings.unique {
        print_sorted(
            lines
                .iter()
                .dedup_by(|a, b| get_months_dedup(a) == get_months_dedup(b)),
            &settings,
        )
    } else if settings.unique {
        print_sorted(
            lines
                .iter()
                .dedup_by(|a, b| get_nums_dedup(a) == get_nums_dedup(b)),
            &settings,
        )
    } else {
        print_sorted(lines.iter(), &settings)
    }

    0
}

fn exec_check_file(lines: Lines<BufReader<Box<dyn Read>>>, settings: &Settings) -> i32 {
    // errors yields the line before each disorder,
    // plus the last line (quirk of .coalesce())
    let unwrapped_lines = lines.filter_map(|maybe_line| {
        if let Ok(line) = maybe_line {
            Some(line)
        } else {
            None
        }
    });
    let mut errors = unwrapped_lines
        .enumerate()
        .coalesce(|(last_i, last_line), (i, line)| {
            if compare_by(&last_line, &line, &settings) == Ordering::Greater {
                Err(((last_i, last_line), (i, line)))
            } else {
                Ok((i, line))
            }
        });
    if let Some((first_error_index, _line)) = errors.next() {
        // Check for a second "error", as .coalesce() always returns the last
        // line, no matter what our merging function does.
        if let Some(_last_line_or_next_error) = errors.next() {
            if !settings.check_silent {
                println!("sort: disorder in line {}", first_error_index);
            };
            1
        } else {
            // first "error" was actually the last line.
            0
        }
    } else {
        // unwrapped_lines was empty. Empty files are defined to be sorted.
        0
    }
}

fn transform(line: &str, settings: &Settings) -> String {
    let mut transformed = line.to_string();
    for transform_fn in &settings.transform_fns {
        transformed = transform_fn(&transformed);
    }

    transformed
}

#[inline(always)]
fn sort_by(lines: &mut Vec<String>, settings: &Settings) {
    lines.par_sort_by(|a, b| compare_by(a, b, &settings))
}

#[inline]
fn compare_by(a: &str, b: &str, settings: &Settings) -> Ordering {
    let (a_transformed, b_transformed): (String, String);
    let (a, b) = if !settings.transform_fns.is_empty() {
        a_transformed = transform(&a, &settings);
        b_transformed = transform(&b, &settings);
        (a_transformed.as_str(), b_transformed.as_str())
    } else {
        (a, b)
    };

    for compare_fn in &settings.compare_fns {
        let cmp: Ordering = compare_fn(a, b, settings);
        if settings.reverse {
            return cmp.reverse();
        } else {
            return cmp;
        };
    }
    Ordering::Equal
}

#[inline(always)]
fn default_compare(a: &str, b: &str, _: &Settings) -> Ordering {
    a.cmp(b)
}

#[inline(always)]
fn last_resort_compare(a: &str, b: &str, x: &Settings) -> Ordering {
    if x.stable || x.unique {
        Ordering::Equal
    } else {
        default_compare(a, b, x)
    }
}

#[inline(always)]
fn leading_num_common(a: &str) -> &str {
    let mut s = "";
    // Strip string
    for c in a.to_uppercase().chars() {
        if !c.is_numeric()
            && !c.is_whitespace()
            && !c.eq(&DECIMAL_PT)
            && !c.eq(&THOUSANDS_SEP)
            && !c.eq(&ENOTATION)
            && !a.chars().nth(0).unwrap_or('\0').eq(&POSITIVE)
            && !a.chars().nth(0).unwrap_or('\0').eq(&NEGATIVE)
        {
            s = a.split(c).next().unwrap_or("");
            break;
        }
        s = a;
    }
    s
}

fn get_leading_num(a: &str) -> &str {
    let mut s = "";
    let b = leading_num_common(a);

    // GNU numeric sort does recognize '+' or 'e' notation so we strip
    for c in b.chars() {
        if c.eq(&ENOTATION) && b.chars().nth(0).unwrap_or('\0').eq(&POSITIVE) {
            s = b.split(c).next().unwrap_or("");
            break;
        }
        s = b;
    }

    // And empty number lines are to be treated as ‘0’ but only for numeric sort
    if s.is_empty() {
        s = "0";
    };
    s
}

fn get_leading_gen(a: &str) -> &str {
    let mut s = leading_num_common(a);

    // Cleanup strips
    let mut p_iter = s.chars().peekable();
    // Checks next char and avoid borrow after move of a for loop
    while let Some(c) = p_iter.next() {
        p_iter.next();
        let next_char_numeric = p_iter.peek().unwrap_or(&'\0').is_numeric();
        // Only general numeric recognizes e notation and the '+' sign
        if c.eq(&ENOTATION) || c.eq(&DECIMAL_PT) && !next_char_numeric {
            s = a.split(c).next().unwrap_or("");
            break;
        } else if c.eq(&POSITIVE) && !next_char_numeric {
            let mut v: Vec<&str> = s.split(c).collect();
            v.split_off(1);
            // 'Let' here avoids returning a value referencing data owned by the current function
            let s = &v.join("");
            break;
        }
    }
    s
}

fn get_months_dedup(a: &str) -> String {
    let pattern = if a.trim().len().ge(&3) {
        // Split at 3rd char and get first element of tuple ".0"
        a.split_at(3).0
    } else {
        ""
    };

    let month = match pattern.to_uppercase().as_ref() {
        "JAN" => Month::January,
        "FEB" => Month::February,
        "MAR" => Month::March,
        "APR" => Month::April,
        "MAY" => Month::May,
        "JUN" => Month::June,
        "JUL" => Month::July,
        "AUG" => Month::August,
        "SEP" => Month::September,
        "OCT" => Month::October,
        "NOV" => Month::November,
        "DEC" => Month::December,
        _ => Month::Unknown,
    };

    if month == Month::Unknown {
        "".to_string()
    } else {
        pattern.to_uppercase().to_string()
    }
}

// *For all dedups/uniques must compare leading numbers*
// Numeric compare and unique is specifically *not* the same as sort | uniq
// See: https://www.gnu.org/software/coreutils/manual/html_node/sort-invocation.html
fn get_nums_dedup(a: &str) -> &str {
    // Trim and remove any leading zeros
    let s = a.trim().trim_start_matches('0');

    // Get first char
    let c = s.chars().nth(0).unwrap_or('\0');

    // Empty lines and non-number lines are treated as the same for dedup
    if s.is_empty() {
        ""
    } else if !c.eq(&NEGATIVE) && !c.is_numeric() {
        ""
    // Prepare lines for comparison of only the numerical leading numbers
    } else {
        get_leading_num(s)
    }
}

/// Parse the beginning string into an f64, returning -inf instead of NaN on errors.
#[inline(always)]
fn permissive_f64_parse(a: &str) -> f64 {
    // Remove thousands seperators
    let a = a.replace(THOUSANDS_SEP, "");

    // GNU sort treats "NaN" as non-number in numeric, so it needs special care.
    // *Keep this trim before parse* despite what POSIX may say about -b and -n
    // because GNU and BSD both seem to require it to match their behavior
    match a.trim().parse::<f64>() {
        Ok(a) if a.is_nan() => std::f64::NEG_INFINITY,
        Ok(a) => a,
        Err(_) => std::f64::NEG_INFINITY,
    }
}

fn numeric_compare(a: &str, b: &str, x: &Settings) -> Ordering {
    #![allow(clippy::comparison_chain)]

    let sa = get_leading_num(a);
    let sb = get_leading_num(b);

    let fa = permissive_f64_parse(sa);
    let fb = permissive_f64_parse(sb);

    // f64::cmp isn't implemented (due to NaN issues); implement directly instead
    if fa > fb {
        Ordering::Greater
    } else if fa < fb {
        Ordering::Less
    } else {
        last_resort_compare(a, b, x)
    }
}

/// Compares two floats, with errors and non-numerics assumed to be -inf.
/// Stops coercing at the first non-numeric char.
fn general_numeric_compare(a: &str, b: &str, x: &Settings) -> Ordering {
    #![allow(clippy::comparison_chain)]

    let sa = get_leading_gen(a);
    let sb = get_leading_gen(b);

    let fa = permissive_f64_parse(sa);
    let fb = permissive_f64_parse(sb);

    // f64::cmp isn't implemented (due to NaN issues); implement directly instead
    if fa > fb {
        Ordering::Greater
    } else if fa < fb {
        Ordering::Less
    } else {
        last_resort_compare(a, b, x)
    }
}

fn human_numeric_convert(a: &str) -> f64 {
    let num_str = get_leading_num(a);
    let (_, suffix) = a.split_at(num_str.len());
    let num_part = permissive_f64_parse(num_str);
    let suffix: f64 = match suffix.parse().unwrap_or('\0') {
        'K' => 1E3,
        'M' => 1E6,
        'G' => 1E9,
        'T' => 1E12,
        'P' => 1E15,
        _ => 1f64,
    };
    num_part * suffix
}

/// Compare two strings as if they are human readable sizes.
/// AKA 1M > 100k
fn human_numeric_size_compare(a: &str, b: &str, x: &Settings) -> Ordering {
    #![allow(clippy::comparison_chain)]
    let fa = human_numeric_convert(a);
    let fb = human_numeric_convert(b);

    // f64::cmp isn't implemented (due to NaN issues); implement directly instead
    if fa > fb {
        Ordering::Greater
    } else if fa < fb {
        Ordering::Less
    } else {
        last_resort_compare(a, b, x)
    }
}

fn get_rand_string() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect::<String>()
}

fn get_hash<T: Hash>(t: &T) -> u64 {
    let mut s: FnvHasher = Default::default();
    t.hash(&mut s);
    s.finish()
}

fn random_shuffle(a: &str, b: &str, x: &Settings) -> Ordering {
    #![allow(clippy::comparison_chain)]
    let salt_slice = x.salt.as_str();

    let da = get_hash(&[a, salt_slice].concat());
    let db = get_hash(&[b, salt_slice].concat());

    da.cmp(&db)
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum Month {
    Unknown,
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

/// Parse the beginning string into a Month, returning Month::Unknown on errors.
fn month_parse(line: &str) -> Month {
    // GNU splits at any 3 letter match "JUNNNN" is JUN
    let pattern = if line.trim().len().ge(&3) {
        // Split a 3 and get first element of tuple ".0"
        line.split_at(3).0
    } else {
        ""
    };

    match pattern.to_uppercase().as_ref() {
        "JAN" => Month::January,
        "FEB" => Month::February,
        "MAR" => Month::March,
        "APR" => Month::April,
        "MAY" => Month::May,
        "JUN" => Month::June,
        "JUL" => Month::July,
        "AUG" => Month::August,
        "SEP" => Month::September,
        "OCT" => Month::October,
        "NOV" => Month::November,
        "DEC" => Month::December,
        _ => Month::Unknown,
    }
}

fn month_compare(a: &str, b: &str, x: &Settings) -> Ordering {
    let ma = month_parse(a);
    let mb = month_parse(b);

    if ma > mb {
        Ordering::Greater
    } else if ma < mb {
        Ordering::Less
    } else {
        last_resort_compare(a, b, x)
    }
}

fn version_compare(a: &str, b: &str, x: &Settings) -> Ordering {
    #![allow(clippy::comparison_chain)]
    let ver_a = Version::parse(a);
    let ver_b = Version::parse(b);
    // Version::cmp is not implemented; implement comparison directly
    if ver_a > ver_b {
        Ordering::Greater
    } else if ver_a < ver_b {
        Ordering::Less
    } else {
        last_resort_compare(a, b, x)
    }
}

fn remove_nondictionary_chars(s: &str) -> String {
    // According to GNU, by default letters and digits are those of ASCII
    // and a blank is a space or a tab
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .collect::<String>()
}

fn remove_nonprinting_chars(s: &str) -> String {
    // However, GNU says nonprinting chars is more permissive.  All ASCII except control chars ie, escape, newline
    s.chars()
        .filter(|c| c.is_ascii() && !c.is_ascii_control())
        .collect::<String>()
}

fn print_sorted<S, T: Iterator<Item = S>>(iter: T, settings: &Settings)
where
    S: std::fmt::Display,
{
    let mut file: Box<dyn Write> = match settings.outfile {
        Some(ref filename) => match File::create(Path::new(&filename)) {
            Ok(f) => Box::new(BufWriter::new(f)) as Box<dyn Write>,
            Err(e) => {
                show_error!("sort: {0}: {1}", filename, e.to_string());
                panic!("Could not open output file");
            }
        },
        None => Box::new(stdout()) as Box<dyn Write>,
    };

    if settings.zero_terminated {
        for line in iter {
            let str = format!("{}\0", line);
            crash_if_err!(1, file.write_all(str.as_bytes()));
        }
    } else {
        for line in iter {
            let str = format!("{}\n", line);
            crash_if_err!(1, file.write_all(str.as_bytes()));
        }
    }
}

// from cat.rs
fn open(path: &str) -> Option<(Box<dyn Read>, bool)> {
    if path == "-" {
        let stdin = stdin();
        return Some((Box::new(stdin) as Box<dyn Read>, is_stdin_interactive()));
    }

    match File::open(Path::new(path)) {
        Ok(f) => Some((Box::new(f) as Box<dyn Read>, false)),
        Err(e) => {
            show_error!("sort: {0}: {1}", path, e.to_string());
            None
        }
    }
}
