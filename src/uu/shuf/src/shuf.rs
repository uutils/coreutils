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

mod nonrepeating_iterator;
mod rand_read_adapter;

use nonrepeating_iterator::NonrepeatingIterator;

enum Mode {
    Default(PathBuf),
    Echo(Vec<OsString>),
    InputRange(RangeInclusive<usize>),
}

const BUF_SIZE: usize = 64 * 1024;

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

    let mut output = BufWriter::with_capacity(
        BUF_SIZE,
        match options.output {
            None => Box::new(stdout()) as Box<dyn OsWrite>,
            Some(ref s) => {
                let file = File::create(s).map_err_context(
                    || translate!("shuf-error-failed-to-open-for-writing", "file" => s.quote()),
                )?;
                Box::new(file) as Box<dyn OsWrite>
            }
        },
    );

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
        // The itoa crate is surprisingly much more efficient than a formatted write.
        // It speeds up `shuf -r -n1000000 -i1-1024` by 1.8×.
        let mut buf = itoa::Buffer::new();
        output.write_all(buf.format(*self).as_bytes())
    }
}

fn shuf_exec(
    input: &mut impl Shufable,
    opts: &Options,
    rng: &mut WrappedRng,
    output: &mut BufWriter<Box<dyn OsWrite>>,
) -> UResult<()> {
    let ctx = || translate!("shuf-error-write-failed");
    let error_cell = rng.get_error_cell();
    if opts.repeat {
        if input.is_empty() {
            return Err(USimpleError::new(
                1,
                translate!("shuf-error-no-lines-to-repeat"),
            ));
        }
        for _ in 0..opts.head_count {
            let r = input.choose(rng);
            WrappedRng::check_error(error_cell.as_ref())?;

            r.write_all_to(output).map_err_context(ctx)?;
            output.write_all(&[opts.sep]).map_err_context(ctx)?;
        }
    } else {
        let shuffled = input.partial_shuffle(rng, opts.head_count);
        WrappedRng::check_error(error_cell.as_ref())?;

        for r in shuffled {
            r.write_all_to(output).map_err_context(ctx)?;
            output.write_all(&[opts.sep]).map_err_context(ctx)?;
        }
    }
    output.flush().map_err_context(ctx)?;

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

impl WrappedRng {
    fn get_error_cell(&self) -> Option<rand_read_adapter::ErrorCell> {
        if let Self::RngFile(adapter) = self {
            Some(adapter.error.clone())
        } else {
            None
        }
    }

    fn check_error(error_cell: Option<&rand_read_adapter::ErrorCell>) -> UResult<()> {
        if let Some(cell) = error_cell {
            if let Some(err) = cell.take() {
                return Err(err.map_err_context(|| translate!("shuf-error-read-random-bytes")));
            }
        }
        Ok(())
    }
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
