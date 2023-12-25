// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) cmdline evec seps rvec fdata

use clap::{crate_version, Arg, ArgAction, Command};
use memchr::memchr_iter;
use rand::prelude::SliceRandom;
use rand::RngCore;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Write};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage};

mod rand_read_adapter;

enum Mode {
    Default(String),
    Echo(Vec<String>),
    InputRange((usize, usize)),
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
    pub static FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let mode = if let Some(args) = matches.get_many::<String>(options::ECHO) {
        Mode::Echo(args.map(String::from).collect())
    } else if let Some(range) = matches.get_one::<String>(options::INPUT_RANGE) {
        match parse_range(range) {
            Ok(m) => Mode::InputRange(m),
            Err(msg) => {
                return Err(USimpleError::new(1, msg));
            }
        }
    } else {
        Mode::Default(
            matches
                .get_one::<String>(options::FILE)
                .map(|s| s.as_str())
                .unwrap_or("-")
                .to_string(),
        )
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

    match mode {
        Mode::Echo(args) => {
            let mut evec = args.iter().map(String::as_bytes).collect::<Vec<_>>();
            find_seps(&mut evec, options.sep);
            shuf_bytes(&mut evec, options)?;
        }
        Mode::InputRange((b, e)) => {
            let rvec = (b..e).map(|x| format!("{x}")).collect::<Vec<String>>();
            let mut rvec = rvec.iter().map(String::as_bytes).collect::<Vec<&[u8]>>();
            shuf_bytes(&mut rvec, options)?;
        }
        Mode::Default(filename) => {
            let fdata = read_input_file(&filename)?;
            let mut fdata = vec![&fdata[..]];
            find_seps(&mut fdata, options.sep);
            shuf_bytes(&mut fdata, options)?;
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
        .args_override_self(true)
        .arg(
            Arg::new(options::ECHO)
                .short('e')
                .long(options::ECHO)
                .value_name("ARG")
                .help("treat each ARG as an input line")
                .use_value_delimiter(false)
                .num_args(0..)
                .conflicts_with(options::INPUT_RANGE),
        )
        .arg(
            Arg::new(options::INPUT_RANGE)
                .short('i')
                .long(options::INPUT_RANGE)
                .value_name("LO-HI")
                .help("treat each number LO through HI as an input line")
                .conflicts_with(options::FILE),
        )
        .arg(
            Arg::new(options::HEAD_COUNT)
                .short('n')
                .long(options::HEAD_COUNT)
                .value_name("COUNT")
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
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(options::FILE).value_hint(clap::ValueHint::FilePath))
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

fn shuf_bytes(input: &mut Vec<&[u8]>, opts: Options) -> UResult<()> {
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

    if input.is_empty() {
        return Ok(());
    }

    if opts.repeat {
        for _ in 0..opts.head_count {
            // Returns None is the slice is empty. We checked this before, so
            // this is safe.
            let r = input.choose(&mut rng).unwrap();

            output
                .write_all(r)
                .map_err_context(|| "write failed".to_string())?;
            output
                .write_all(&[opts.sep])
                .map_err_context(|| "write failed".to_string())?;
        }
    } else {
        let (shuffled, _) = input.partial_shuffle(&mut rng, opts.head_count);
        for r in shuffled {
            output
                .write_all(r)
                .map_err_context(|| "write failed".to_string())?;
            output
                .write_all(&[opts.sep])
                .map_err_context(|| "write failed".to_string())?;
        }
    }

    Ok(())
}

fn parse_range(input_range: &str) -> Result<(usize, usize), String> {
    if let Some((from, to)) = input_range.split_once('-') {
        let begin = from
            .parse::<usize>()
            .map_err(|_| format!("invalid input range: {}", from.quote()))?;
        let end = to
            .parse::<usize>()
            .map_err(|_| format!("invalid input range: {}", to.quote()))?;
        Ok((begin, end + 1))
    } else {
        Err(format!("invalid input range: {}", input_range.quote()))
    }
}

fn parse_head_count(headcounts: Vec<String>) -> Result<usize, String> {
    let mut result = std::usize::MAX;
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
