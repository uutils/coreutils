// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) cmdline evec nonrepeating seps shufable rvec fdata

use clap::{crate_version, Arg, ArgAction, Command};
use memchr::memchr_iter;
use rand::prelude::SliceRandom;
use rand::{Rng, RngCore};
use std::collections::HashSet;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Error, Read, Write};
use std::ops::RangeInclusive;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::{format_usage, help_about, help_usage};

mod rand_read_adapter;

enum Mode {
    Default(String),
    Echo(Vec<String>),
    InputRange(RangeInclusive<usize>),
}

static USAGE: &str = help_usage!("shuf.md");
static ABOUT: &str = help_about!("shuf.md");

struct Options {
    head_count: usize,
    output: Option<String>,
    random_source: Option<String>,
    repeat: bool,
    sep: u8,
}

mod options {
    pub static ECHO: &str = "echo";
    pub static INPUT_RANGE: &str = "input-range";
    pub static HEAD_COUNT: &str = "head-count";
    pub static OUTPUT: &str = "output";
    pub static RANDOM_SOURCE: &str = "random-source";
    pub static REPEAT: &str = "repeat";
    pub static ZERO_TERMINATED: &str = "zero-terminated";
    pub static FILE_OR_ARGS: &str = "file-or-args";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let mode = if matches.get_flag(options::ECHO) {
        Mode::Echo(
            matches
                .get_many::<String>(options::FILE_OR_ARGS)
                .unwrap_or_default()
                .map(String::from)
                .collect(),
        )
    } else if let Some(range) = matches.get_one::<String>(options::INPUT_RANGE) {
        match parse_range(range) {
            Ok(m) => Mode::InputRange(m),
            Err(msg) => {
                return Err(USimpleError::new(1, msg));
            }
        }
    } else {
        let mut operands = matches
            .get_many::<String>(options::FILE_OR_ARGS)
            .unwrap_or_default();
        let file = operands.next().cloned().unwrap_or("-".into());
        if let Some(second_file) = operands.next() {
            return Err(UUsageError::new(
                1,
                format!("unexpected argument '{second_file}' found"),
            ));
        };
        Mode::Default(file)
    };

    let options = Options {
        head_count: {
            let headcounts = matches
                .get_many::<String>(options::HEAD_COUNT)
                .unwrap_or_default()
                .cloned()
                .collect();
            match parse_head_count(headcounts) {
                Ok(val) => val,
                Err(msg) => return Err(USimpleError::new(1, msg)),
            }
        },
        output: matches.get_one::<String>(options::OUTPUT).map(String::from),
        random_source: matches
            .get_one::<String>(options::RANDOM_SOURCE)
            .map(String::from),
        repeat: matches.get_flag(options::REPEAT),
        sep: if matches.get_flag(options::ZERO_TERMINATED) {
            0x00_u8
        } else {
            0x0a_u8
        },
    };

    if options.head_count == 0 {
        // Do not attempt to read the random source or the input file.
        // However, we must touch the output file, if given:
        if let Some(s) = options.output {
            File::create(&s[..])
                .map_err_context(|| format!("failed to open {} for writing", s.quote()))?;
        }
        return Ok(());
    }

    match mode {
        Mode::Echo(args) => {
            let mut evec = args.iter().map(String::as_bytes).collect::<Vec<_>>();
            find_seps(&mut evec, options.sep);
            shuf_exec(&mut evec, options)?;
        }
        Mode::InputRange(mut range) => {
            shuf_exec(&mut range, options)?;
        }
        Mode::Default(filename) => {
            let fdata = read_input_file(&filename)?;
            let mut fdata = vec![&fdata[..]];
            find_seps(&mut fdata, options.sep);
            shuf_exec(&mut fdata, options)?;
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ECHO)
                .short('e')
                .long(options::ECHO)
                .help("treat each ARG as an input line")
                .action(clap::ArgAction::SetTrue)
                .overrides_with(options::ECHO)
                .conflicts_with(options::INPUT_RANGE),
        )
        .arg(
            Arg::new(options::INPUT_RANGE)
                .short('i')
                .long(options::INPUT_RANGE)
                .value_name("LO-HI")
                .help("treat each number LO through HI as an input line")
                .conflicts_with(options::FILE_OR_ARGS),
        )
        .arg(
            Arg::new(options::HEAD_COUNT)
                .short('n')
                .long(options::HEAD_COUNT)
                .value_name("COUNT")
                .action(clap::ArgAction::Append)
                .help("output at most COUNT lines"),
        )
        .arg(
            Arg::new(options::OUTPUT)
                .short('o')
                .long(options::OUTPUT)
                .value_name("FILE")
                .help("write result to FILE instead of standard output")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::RANDOM_SOURCE)
                .long(options::RANDOM_SOURCE)
                .value_name("FILE")
                .help("get random bytes from FILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::REPEAT)
                .short('r')
                .long(options::REPEAT)
                .help("output lines can be repeated")
                .action(ArgAction::SetTrue)
                .overrides_with(options::REPEAT),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue)
                .overrides_with(options::ZERO_TERMINATED),
        )
        .arg(
            Arg::new(options::FILE_OR_ARGS)
                .action(clap::ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn read_input_file(filename: &str) -> UResult<Vec<u8>> {
    let mut file = BufReader::new(if filename == "-" {
        Box::new(stdin()) as Box<dyn Read>
    } else {
        let file = File::open(filename)
            .map_err_context(|| format!("failed to open {}", filename.quote()))?;
        Box::new(file) as Box<dyn Read>
    });

    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err_context(|| format!("failed reading {}", filename.quote()))?;

    Ok(data)
}

fn find_seps(data: &mut Vec<&[u8]>, sep: u8) {
    // Special case: If data is empty (and does not even contain a single 'sep'
    // to indicate the presence of the empty element), then behave as if the input contained no elements at all.
    if data.len() == 1 && data[0].is_empty() {
        data.clear();
        return;
    }

    // need to use for loop so we don't borrow the vector as we modify it in place
    // basic idea:
    // * We don't care about the order of the result. This lets us slice the slices
    //   without making a new vector.
    // * Starting from the end of the vector, we examine each element.
    // * If that element contains the separator, we remove it from the vector,
    //   and then sub-slice it into slices that do not contain the separator.
    // * We maintain the invariant throughout that each element in the vector past
    //   the ith element does not have any separators remaining.
    for i in (0..data.len()).rev() {
        if data[i].contains(&sep) {
            let this = data.swap_remove(i);
            let mut p = 0;
            for i in memchr_iter(sep, this) {
                data.push(&this[p..i]);
                p = i + 1;
            }
            if p < this.len() {
                data.push(&this[p..]);
            }
        }
    }
}

trait Shufable {
    type Item: Writable;
    fn is_empty(&self) -> bool;
    fn choose(&self, rng: &mut WrappedRng) -> Self::Item;
    // This type shouldn't even be known. However, because we want to support
    // Rust 1.70, it is not possible to return "impl Iterator".
    // TODO: When the MSRV is raised, rewrite this to return "impl Iterator".
    type PartialShuffleIterator<'b>: Iterator<Item = Self::Item>
    where
        Self: 'b;
    fn partial_shuffle<'b>(
        &'b mut self,
        rng: &'b mut WrappedRng,
        amount: usize,
    ) -> Self::PartialShuffleIterator<'b>;
}

impl<'a> Shufable for Vec<&'a [u8]> {
    type Item = &'a [u8];
    fn is_empty(&self) -> bool {
        (**self).is_empty()
    }
    fn choose(&self, rng: &mut WrappedRng) -> Self::Item {
        // Note: "copied()" only copies the reference, not the entire [u8].
        // Returns None if the slice is empty. We checked this before, so
        // this is safe.
        (**self).choose(rng).unwrap()
    }
    type PartialShuffleIterator<'b> = std::iter::Copied<std::slice::Iter<'b, &'a [u8]>> where Self: 'b;
    fn partial_shuffle<'b>(
        &'b mut self,
        rng: &'b mut WrappedRng,
        amount: usize,
    ) -> Self::PartialShuffleIterator<'b> {
        // Note: "copied()" only copies the reference, not the entire [u8].
        (**self).partial_shuffle(rng, amount).0.iter().copied()
    }
}

impl Shufable for RangeInclusive<usize> {
    type Item = usize;
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
    fn choose(&self, rng: &mut WrappedRng) -> usize {
        rng.gen_range(self.clone())
    }
    type PartialShuffleIterator<'b> = NonrepeatingIterator<'b> where Self: 'b;
    fn partial_shuffle<'b>(
        &'b mut self,
        rng: &'b mut WrappedRng,
        amount: usize,
    ) -> Self::PartialShuffleIterator<'b> {
        NonrepeatingIterator::new(self.clone(), rng, amount)
    }
}

enum NumberSet {
    AlreadyListed(HashSet<usize>),
    Remaining(Vec<usize>),
}

struct NonrepeatingIterator<'a> {
    range: RangeInclusive<usize>,
    rng: &'a mut WrappedRng,
    remaining_count: usize,
    buf: NumberSet,
}

impl<'a> NonrepeatingIterator<'a> {
    fn new(range: RangeInclusive<usize>, rng: &'a mut WrappedRng, amount: usize) -> Self {
        let capped_amount = if range.start() > range.end() {
            0
        } else if *range.start() == 0 && *range.end() == usize::MAX {
            amount
        } else {
            amount.min(range.end() - range.start() + 1)
        };
        NonrepeatingIterator {
            range,
            rng,
            remaining_count: capped_amount,
            buf: NumberSet::AlreadyListed(HashSet::default()),
        }
    }

    fn produce(&mut self) -> usize {
        debug_assert!(self.range.start() <= self.range.end());
        match &mut self.buf {
            NumberSet::AlreadyListed(already_listed) => {
                let chosen = loop {
                    let guess = self.rng.gen_range(self.range.clone());
                    let newly_inserted = already_listed.insert(guess);
                    if newly_inserted {
                        break guess;
                    }
                };
                // Once a significant fraction of the interval has already been enumerated,
                // the number of attempts to find a number that hasn't been chosen yet increases.
                // Therefore, we need to switch at some point from "set of already returned values" to "list of remaining values".
                let range_size = (self.range.end() - self.range.start()).saturating_add(1);
                if number_set_should_list_remaining(already_listed.len(), range_size) {
                    let mut remaining = self
                        .range
                        .clone()
                        .filter(|n| !already_listed.contains(n))
                        .collect::<Vec<_>>();
                    assert!(remaining.len() >= self.remaining_count);
                    remaining.partial_shuffle(&mut self.rng, self.remaining_count);
                    remaining.truncate(self.remaining_count);
                    self.buf = NumberSet::Remaining(remaining);
                }
                chosen
            }
            NumberSet::Remaining(remaining_numbers) => {
                debug_assert!(!remaining_numbers.is_empty());
                // We only enter produce() when there is at least one actual element remaining, so popping must always return an element.
                remaining_numbers.pop().unwrap()
            }
        }
    }
}

impl<'a> Iterator for NonrepeatingIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.range.is_empty() || self.remaining_count == 0 {
            return None;
        }
        self.remaining_count -= 1;
        Some(self.produce())
    }
}

// This could be a method, but it is much easier to test as a stand-alone function.
fn number_set_should_list_remaining(listed_count: usize, range_size: usize) -> bool {
    // Arbitrarily determine the switchover point to be around 25%. This is because:
    // - HashSet has a large space overhead for the hash table load factor.
    // - This means that somewhere between 25-40%, the memory required for a "positive" HashSet and a "negative" Vec should be the same.
    // - HashSet has a small but non-negligible overhead for each lookup, so we have a slight preference for Vec anyway.
    // - At 25%, on average 1.33 attempts are needed to find a number that hasn't been taken yet.
    // - Finally, "24%" is computationally the simplest:
    listed_count >= range_size / 4
}

trait Writable {
    fn write_all_to(&self, output: &mut impl Write) -> Result<(), Error>;
}

impl<'a> Writable for &'a [u8] {
    fn write_all_to(&self, output: &mut impl Write) -> Result<(), Error> {
        output.write_all(self)
    }
}

impl Writable for usize {
    fn write_all_to(&self, output: &mut impl Write) -> Result<(), Error> {
        output.write_all(format!("{self}").as_bytes())
    }
}

fn shuf_exec(input: &mut impl Shufable, opts: Options) -> UResult<()> {
    let mut output = BufWriter::new(match opts.output {
        None => Box::new(stdout()) as Box<dyn Write>,
        Some(s) => {
            let file = File::create(&s[..])
                .map_err_context(|| format!("failed to open {} for writing", s.quote()))?;
            Box::new(file) as Box<dyn Write>
        }
    });

    let mut rng = match opts.random_source {
        Some(r) => {
            let file = File::open(&r[..])
                .map_err_context(|| format!("failed to open random source {}", r.quote()))?;
            WrappedRng::RngFile(rand_read_adapter::ReadRng::new(file))
        }
        None => WrappedRng::RngDefault(rand::thread_rng()),
    };

    if opts.repeat {
        if input.is_empty() {
            return Err(USimpleError::new(1, "no lines to repeat"));
        }
        for _ in 0..opts.head_count {
            let r = input.choose(&mut rng);

            r.write_all_to(&mut output)
                .map_err_context(|| "write failed".to_string())?;
            output
                .write_all(&[opts.sep])
                .map_err_context(|| "write failed".to_string())?;
        }
    } else {
        let shuffled = input.partial_shuffle(&mut rng, opts.head_count);
        for r in shuffled {
            r.write_all_to(&mut output)
                .map_err_context(|| "write failed".to_string())?;
            output
                .write_all(&[opts.sep])
                .map_err_context(|| "write failed".to_string())?;
        }
    }

    Ok(())
}

fn parse_range(input_range: &str) -> Result<RangeInclusive<usize>, String> {
    if let Some((from, to)) = input_range.split_once('-') {
        let begin = from
            .parse::<usize>()
            .map_err(|_| format!("invalid input range: {}", from.quote()))?;
        let end = to
            .parse::<usize>()
            .map_err(|_| format!("invalid input range: {}", to.quote()))?;
        if begin <= end || begin == end + 1 {
            Ok(begin..=end)
        } else {
            Err(format!("invalid input range: {}", input_range.quote()))
        }
    } else {
        Err(format!("invalid input range: {}", input_range.quote()))
    }
}

fn parse_head_count(headcounts: Vec<String>) -> Result<usize, String> {
    let mut result = usize::MAX;
    for count in headcounts {
        match count.parse::<usize>() {
            Ok(pv) => result = std::cmp::min(result, pv),
            Err(_) => return Err(format!("invalid line count: {}", count.quote())),
        }
    }
    Ok(result)
}

enum WrappedRng {
    RngFile(rand_read_adapter::ReadRng<File>),
    RngDefault(rand::rngs::ThreadRng),
}

impl RngCore for WrappedRng {
    fn next_u32(&mut self) -> u32 {
        match self {
            Self::RngFile(r) => r.next_u32(),
            Self::RngDefault(r) => r.next_u32(),
        }
    }

    fn next_u64(&mut self) -> u64 {
        match self {
            Self::RngFile(r) => r.next_u64(),
            Self::RngDefault(r) => r.next_u64(),
        }
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        match self {
            Self::RngFile(r) => r.fill_bytes(dest),
            Self::RngDefault(r) => r.fill_bytes(dest),
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        match self {
            Self::RngFile(r) => r.try_fill_bytes(dest),
            Self::RngDefault(r) => r.try_fill_bytes(dest),
        }
    }
}

#[cfg(test)]
// Since the computed value is a bool, it is more readable to write the expected value out:
#[allow(clippy::bool_assert_comparison)]
mod test_number_set_decision {
    use super::number_set_should_list_remaining;

    #[test]
    fn test_stay_positive_large_remaining_first() {
        assert_eq!(false, number_set_should_list_remaining(0, usize::MAX));
    }

    #[test]
    fn test_stay_positive_large_remaining_second() {
        assert_eq!(false, number_set_should_list_remaining(1, usize::MAX));
    }

    #[test]
    fn test_stay_positive_large_remaining_tenth() {
        assert_eq!(false, number_set_should_list_remaining(9, usize::MAX));
    }

    #[test]
    fn test_stay_positive_smallish_range_first() {
        assert_eq!(false, number_set_should_list_remaining(0, 12345));
    }

    #[test]
    fn test_stay_positive_smallish_range_second() {
        assert_eq!(false, number_set_should_list_remaining(1, 12345));
    }

    #[test]
    fn test_stay_positive_smallish_range_tenth() {
        assert_eq!(false, number_set_should_list_remaining(9, 12345));
    }

    #[test]
    fn test_stay_positive_small_range_not_too_early() {
        assert_eq!(false, number_set_should_list_remaining(1, 10));
    }

    // Don't want to test close to the border, in case we decide to change the threshold.
    // However, at 50% coverage, we absolutely should switch:
    #[test]
    fn test_switch_half() {
        assert_eq!(true, number_set_should_list_remaining(1234, 2468));
    }

    // Ensure that the decision is monotonous:
    #[test]
    fn test_switch_late1() {
        assert_eq!(true, number_set_should_list_remaining(12340, 12345));
    }

    #[test]
    fn test_switch_late2() {
        assert_eq!(true, number_set_should_list_remaining(12344, 12345));
    }

    // Ensure that we are overflow-free:
    #[test]
    fn test_no_crash_exceed_max_size1() {
        assert_eq!(false, number_set_should_list_remaining(12345, usize::MAX));
    }

    #[test]
    fn test_no_crash_exceed_max_size2() {
        assert_eq!(
            true,
            number_set_should_list_remaining(usize::MAX - 1, usize::MAX)
        );
    }

    #[test]
    fn test_no_crash_exceed_max_size3() {
        assert_eq!(
            true,
            number_set_should_list_remaining(usize::MAX, usize::MAX)
        );
    }
}
