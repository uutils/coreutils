//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) cmdline evec seps rvec fdata

use clap::{crate_version, App, Arg};
use rand::Rng;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Write};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::InvalidEncodingHandling;

enum Mode {
    Default(String),
    Echo(Vec<String>),
    InputRange((usize, usize)),
}

static NAME: &str = "shuf";
static USAGE: &str = r#"shuf [OPTION]... [FILE]
  or:  shuf -e [OPTION]... [ARG]...
  or:  shuf -i LO-HI [OPTION]...
Write a random permutation of the input lines to standard output.

With no FILE, or when FILE is -, read standard input.
"#;
static TEMPLATE: &str = "Usage: {usage}\nMandatory arguments to long options are mandatory for short options too.\n{unified}";

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

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let mode = if let Some(args) = matches.values_of(options::ECHO) {
        Mode::Echo(args.map(String::from).collect())
    } else if let Some(range) = matches.value_of(options::INPUT_RANGE) {
        match parse_range(range) {
            Ok(m) => Mode::InputRange(m),
            Err(msg) => {
                return Err(USimpleError::new(1, msg));
            }
        }
    } else {
        Mode::Default(matches.value_of(options::FILE).unwrap_or("-").to_string())
    };

    let options = Options {
        head_count: match matches.value_of(options::HEAD_COUNT) {
            Some(count) => match count.parse::<usize>() {
                Ok(val) => val,
                Err(_) => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid line count: {}", count.quote()),
                    ));
                }
            },
            None => std::usize::MAX,
        },
        output: matches.value_of(options::OUTPUT).map(String::from),
        random_source: matches.value_of(options::RANDOM_SOURCE).map(String::from),
        repeat: matches.is_present(options::REPEAT),
        sep: if matches.is_present(options::ZERO_TERMINATED) {
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
            let rvec = (b..e).map(|x| format!("{}", x)).collect::<Vec<String>>();
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

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .template(TEMPLATE)
        .usage(USAGE)
        .arg(
            Arg::with_name(options::ECHO)
                .short("e")
                .long(options::ECHO)
                .takes_value(true)
                .value_name("ARG")
                .help("treat each ARG as an input line")
                .multiple(true)
                .use_delimiter(false)
                .min_values(0)
                .conflicts_with(options::INPUT_RANGE),
        )
        .arg(
            Arg::with_name(options::INPUT_RANGE)
                .short("i")
                .long(options::INPUT_RANGE)
                .takes_value(true)
                .value_name("LO-HI")
                .help("treat each number LO through HI as an input line")
                .conflicts_with(options::FILE),
        )
        .arg(
            Arg::with_name(options::HEAD_COUNT)
                .short("n")
                .long(options::HEAD_COUNT)
                .takes_value(true)
                .value_name("COUNT")
                .help("output at most COUNT lines"),
        )
        .arg(
            Arg::with_name(options::OUTPUT)
                .short("o")
                .long(options::OUTPUT)
                .takes_value(true)
                .value_name("FILE")
                .help("write result to FILE instead of standard output"),
        )
        .arg(
            Arg::with_name(options::RANDOM_SOURCE)
                .long(options::RANDOM_SOURCE)
                .takes_value(true)
                .value_name("FILE")
                .help("get random bytes from FILE"),
        )
        .arg(
            Arg::with_name(options::REPEAT)
                .short("r")
                .long(options::REPEAT)
                .help("output lines can be repeated"),
        )
        .arg(
            Arg::with_name(options::ZERO_TERMINATED)
                .short("z")
                .long(options::ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline"),
        )
        .arg(Arg::with_name(options::FILE).takes_value(true))
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
            let mut i = 1;
            loop {
                if i == this.len() {
                    break;
                }

                if this[i] == sep {
                    data.push(&this[p..i]);
                    p = i + 1;
                }
                i += 1;
            }
            if p < this.len() {
                data.push(&this[p..i]);
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
            WrappedRng::RngFile(rand::rngs::adapter::ReadRng::new(file))
        }
        None => WrappedRng::RngDefault(rand::thread_rng()),
    };

    // we're generating a random usize. To keep things fair, we take this number mod ceil(log2(length+1))
    let mut len_mod = 1;
    let mut len = input.len();
    while len > 0 {
        len >>= 1;
        len_mod <<= 1;
    }

    let mut count = opts.head_count;
    while count > 0 && !input.is_empty() {
        let mut r = input.len();
        while r >= input.len() {
            r = rng.next_usize() % len_mod;
        }

        // write the randomly chosen value and the separator
        output
            .write_all(input[r])
            .map_err_context(|| "write failed".to_string())?;
        output
            .write_all(&[opts.sep])
            .map_err_context(|| "write failed".to_string())?;

        // if we do not allow repeats, remove the chosen value from the input vector
        if !opts.repeat {
            // shrink the mask if we will drop below a power of 2
            if input.len() % 2 == 0 && len_mod > 2 {
                len_mod >>= 1;
            }
            input.swap_remove(r);
        }

        count -= 1;
    }
    Ok(())
}

fn parse_range(input_range: &str) -> Result<(usize, usize), String> {
    let split: Vec<&str> = input_range.split('-').collect();
    if split.len() != 2 {
        Err(format!("invalid input range: {}", input_range.quote()))
    } else {
        let begin = split[0]
            .parse::<usize>()
            .map_err(|_| format!("invalid input range: {}", split[0].quote()))?;
        let end = split[1]
            .parse::<usize>()
            .map_err(|_| format!("invalid input range: {}", split[1].quote()))?;
        Ok((begin, end + 1))
    }
}

enum WrappedRng {
    RngFile(rand::rngs::adapter::ReadRng<File>),
    RngDefault(rand::rngs::ThreadRng),
}

impl WrappedRng {
    fn next_usize(&mut self) -> usize {
        match *self {
            WrappedRng::RngFile(ref mut r) => r.gen(),
            WrappedRng::RngDefault(ref mut r) => r.gen(),
        }
    }
}
