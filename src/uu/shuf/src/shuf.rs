// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) cmdline evec nonrepeating seps shufable rvec fdata

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command};
use rand::prelude::SliceRandom;
use rand::seq::IndexedRandom;
use rand::{Rng, RngCore};
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufWriter, Error, Read, Write, stdin, stdout};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uucore::display::{OsWrite, Quotable};
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::translate;

mod rand_read_adapter;

enum Mode {
    Default(PathBuf),
    Echo(Vec<OsString>),
    InputRange(RangeInclusive<usize>),
}

struct Options {
    head_count: usize,
    output: Option<PathBuf>,
    random_source: Option<PathBuf>,
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
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let mode = if matches.get_flag(options::ECHO) {
        Mode::Echo(
            matches
                .get_many(options::FILE_OR_ARGS)
                .unwrap_or_default()
                .cloned()
                .collect(),
        )
    } else if let Some(range) = matches.get_one(options::INPUT_RANGE).cloned() {
        Mode::InputRange(range)
    } else {
        let mut operands = matches
            .get_many::<OsString>(options::FILE_OR_ARGS)
            .unwrap_or_default();
        let file = operands.next().cloned().unwrap_or("-".into());
        if let Some(second_file) = operands.next() {
            return Err(UUsageError::new(
                1,
                translate!("shuf-error-unexpected-argument", "arg" => second_file.quote()),
            ));
        }
        Mode::Default(file.into())
    };

    let options = Options {
        // GNU shuf takes the lowest value passed, so we imitate that.
        // It's probably a bug or an implementation artifact though.
        // Busybox takes the final value which is more typical: later
        // options override earlier options.
        head_count: matches
            .get_many::<usize>(options::HEAD_COUNT)
            .unwrap_or_default()
            .copied()
            .min()
            .unwrap_or(usize::MAX),
        output: matches.get_one(options::OUTPUT).cloned(),
        random_source: matches.get_one(options::RANDOM_SOURCE).cloned(),
        repeat: matches.get_flag(options::REPEAT),
        sep: if matches.get_flag(options::ZERO_TERMINATED) {
            b'\0'
        } else {
            b'\n'
        },
    };

    let mut output = BufWriter::new(match options.output {
        None => Box::new(stdout()) as Box<dyn OsWrite>,
        Some(ref s) => {
            let file = File::create(s).map_err_context(
                || translate!("shuf-error-failed-to-open-for-writing", "file" => s.quote()),
            )?;
            Box::new(file) as Box<dyn OsWrite>
        }
    });

    if options.head_count == 0 {
        // In this case we do want to touch the output file but we can quit immediately.
        return Ok(());
    }

    let mut rng = match options.random_source {
        Some(ref r) => {
            let file = File::open(r).map_err_context(
                || translate!("shuf-error-failed-to-open-random-source", "file" => r.quote()),
            )?;
            WrappedRng::RngFile(rand_read_adapter::ReadRng::new(file))
        }
        None => WrappedRng::RngDefault(rand::rng()),
    };

    match mode {
        Mode::Echo(args) => {
            let mut evec: Vec<&OsStr> = args.iter().map(AsRef::as_ref).collect();
            shuf_exec(&mut evec, &options, &mut rng, &mut output)?;
        }
        Mode::InputRange(mut range) => {
            shuf_exec(&mut range, &options, &mut rng, &mut output)?;
        }
        Mode::Default(filename) => {
            let fdata = read_input_file(&filename)?;
            let mut items = split_seps(&fdata, options.sep);
            shuf_exec(&mut items, &options, &mut rng, &mut output)?;
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(translate!("shuf-about"))
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("shuf-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ECHO)
                .short('e')
                .long(options::ECHO)
                .help(translate!("shuf-help-echo"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::ECHO)
                .conflicts_with(options::INPUT_RANGE),
        )
        .arg(
            Arg::new(options::INPUT_RANGE)
                .short('i')
                .long(options::INPUT_RANGE)
                .value_name("LO-HI")
                .help(translate!("shuf-help-input-range"))
                .value_parser(parse_range)
                .conflicts_with(options::FILE_OR_ARGS),
        )
        .arg(
            Arg::new(options::HEAD_COUNT)
                .short('n')
                .long(options::HEAD_COUNT)
                .value_name("COUNT")
                .action(ArgAction::Append)
                .help(translate!("shuf-help-head-count"))
                .value_parser(usize::from_str),
        )
        .arg(
            Arg::new(options::OUTPUT)
                .short('o')
                .long(options::OUTPUT)
                .value_name("FILE")
                .help(translate!("shuf-help-output"))
                .value_parser(ValueParser::path_buf())
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::RANDOM_SOURCE)
                .long(options::RANDOM_SOURCE)
                .value_name("FILE")
                .help(translate!("shuf-help-random-source"))
                .value_parser(ValueParser::path_buf())
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::REPEAT)
                .short('r')
                .long(options::REPEAT)
                .help(translate!("shuf-help-repeat"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::REPEAT),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help(translate!("shuf-help-zero-terminated"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::ZERO_TERMINATED),
        )
        .arg(
            Arg::new(options::FILE_OR_ARGS)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn read_input_file(filename: &Path) -> UResult<Vec<u8>> {
    if filename.as_os_str() == "-" {
        let mut data = Vec::new();
        stdin()
            .read_to_end(&mut data)
            .map_err_context(|| translate!("shuf-error-read-error"))?;
        Ok(data)
    } else {
        std::fs::read(filename).map_err_context(|| filename.maybe_quote().to_string())
    }
}

fn split_seps(data: &[u8], sep: u8) -> Vec<&[u8]> {
    // A single trailing separator is ignored.
    // If data is empty (and does not even contain a single 'sep'
    // to indicate the presence of an empty element), then behave
    // as if the input contained no elements at all.
    let mut elements: Vec<&[u8]> = data.split(|&b| b == sep).collect();
    if elements.last().is_some_and(|e| e.is_empty()) {
        elements.pop();
    }
    elements
}

trait Shufable {
    type Item: Writable;
    fn is_empty(&self) -> bool;
    fn choose(&self, rng: &mut WrappedRng) -> Self::Item;
    fn partial_shuffle<'b>(
        &'b mut self,
        rng: &'b mut WrappedRng,
        amount: usize,
    ) -> impl Iterator<Item = Self::Item>;
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

    fn partial_shuffle<'b>(
        &'b mut self,
        rng: &'b mut WrappedRng,
        amount: usize,
    ) -> impl Iterator<Item = Self::Item> {
        // Note: "copied()" only copies the reference, not the entire [u8].
        (**self).partial_shuffle(rng, amount).0.iter().copied()
    }
}

impl<'a> Shufable for Vec<&'a OsStr> {
    type Item = &'a OsStr;

    fn is_empty(&self) -> bool {
        (**self).is_empty()
    }

    fn choose(&self, rng: &mut WrappedRng) -> Self::Item {
        (**self).choose(rng).unwrap()
    }

    fn partial_shuffle<'b>(
        &'b mut self,
        rng: &'b mut WrappedRng,
        amount: usize,
    ) -> impl Iterator<Item = Self::Item> {
        (**self).partial_shuffle(rng, amount).0.iter().copied()
    }
}

impl Shufable for RangeInclusive<usize> {
    type Item = usize;

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn choose(&self, rng: &mut WrappedRng) -> usize {
        rng.random_range(self.clone())
    }

    fn partial_shuffle<'b>(
        &'b mut self,
        rng: &'b mut WrappedRng,
        amount: usize,
    ) -> impl Iterator<Item = Self::Item> {
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
        } else if range == (0..=usize::MAX) {
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
                    let guess = self.rng.random_range(self.range.clone());
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

impl Iterator for NonrepeatingIterator<'_> {
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
    fn write_all_to(&self, output: &mut impl OsWrite) -> Result<(), Error>;
}

impl Writable for &[u8] {
    fn write_all_to(&self, output: &mut impl OsWrite) -> Result<(), Error> {
        output.write_all(self)
    }
}

impl Writable for &OsStr {
    fn write_all_to(&self, output: &mut impl OsWrite) -> Result<(), Error> {
        output.write_all_os(self)
    }
}

impl Writable for usize {
    fn write_all_to(&self, output: &mut impl OsWrite) -> Result<(), Error> {
        let mut n = *self;

        // Handle the zero case explicitly
        if n == 0 {
            return output.write_all(b"0");
        }

        // Maximum number of digits for u64 is 20 (18446744073709551615)
        let mut buf = [0u8; 20];
        let mut i = 20;

        // Write digits from right to left
        while n > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }

        // Write the relevant part of the buffer to output
        output.write_all(&buf[i..])
    }
}

fn shuf_exec(
    input: &mut impl Shufable,
    opts: &Options,
    rng: &mut WrappedRng,
    output: &mut BufWriter<Box<dyn OsWrite>>,
) -> UResult<()> {
    let ctx = || translate!("shuf-error-write-failed");

    if opts.repeat {
        if input.is_empty() {
            return Err(USimpleError::new(
                1,
                translate!("shuf-error-no-lines-to-repeat"),
            ));
        }
        for _ in 0..opts.head_count {
            let r = input.choose(rng);

            r.write_all_to(output).map_err_context(ctx)?;
            output.write_all(&[opts.sep]).map_err_context(ctx)?;
        }
    } else {
        let shuffled = input.partial_shuffle(rng, opts.head_count);
        for r in shuffled {
            r.write_all_to(output).map_err_context(ctx)?;
            output.write_all(&[opts.sep]).map_err_context(ctx)?;
        }
    }

    Ok(())
}

fn parse_range(input_range: &str) -> Result<RangeInclusive<usize>, String> {
    if let Some((from, to)) = input_range.split_once('-') {
        let begin = from.parse::<usize>().map_err(|e| e.to_string())?;
        let end = to.parse::<usize>().map_err(|e| e.to_string())?;
        if begin <= end || begin == end + 1 {
            Ok(begin..=end)
        } else {
            Err(translate!("shuf-error-start-exceeds-end"))
        }
    } else {
        Err(translate!("shuf-error-missing-dash"))
    }
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
}

#[cfg(test)]
mod test_split_seps {
    use super::split_seps;

    #[test]
    fn test_empty_input() {
        assert!(split_seps(b"", b'\n').is_empty());
    }

    #[test]
    fn test_single_blank_line() {
        assert_eq!(split_seps(b"\n", b'\n'), &[b""]);
    }

    #[test]
    fn test_with_trailing() {
        assert_eq!(split_seps(b"a\nb\nc\n", b'\n'), &[b"a", b"b", b"c"]);
    }

    #[test]
    fn test_without_trailing() {
        assert_eq!(split_seps(b"a\nb\nc", b'\n'), &[b"a", b"b", b"c"]);
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
