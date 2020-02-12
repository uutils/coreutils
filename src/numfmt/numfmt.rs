#![crate_name = "uu_numfmt"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Yury Krivopalov <ykrivopalov@yandex.ru>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate regex;

use getopts::{Matches, Options};
use regex::Regex;
use std::fmt;
use std::io::BufRead;

static NAME: &str = "numfmt";
static VERSION: &str = env!("CARGO_PKG_VERSION");

const IEC_BASES: [f64; 10] = [
    //premature optimization
    1.,
    1024.,
    1048576.,
    1073741824.,
    1099511627776.,
    1125899906842624.,
    1152921504606846976.,
    1180591620717411303424.,
    1208925819614629174706176.,
    1237940039285380274899124224.,
];

type Result<T> = std::result::Result<T, String>;

type WithI = bool;

enum Unit {
    Auto,
    Si,
    Iec(WithI),
    None,
}

enum RawSuffix {
    K,
    M,
    G,
    T,
    P,
    E,
    Z,
    Y,
}

type Suffix = (RawSuffix, WithI);

struct DisplayableSuffix(Suffix);

impl fmt::Display for DisplayableSuffix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let DisplayableSuffix((ref raw_suffix, ref with_i)) = *self;
        match raw_suffix {
            RawSuffix::K => write!(f, "K"),
            RawSuffix::M => write!(f, "M"),
            RawSuffix::G => write!(f, "G"),
            RawSuffix::T => write!(f, "T"),
            RawSuffix::P => write!(f, "P"),
            RawSuffix::E => write!(f, "E"),
            RawSuffix::Z => write!(f, "Z"),
            RawSuffix::Y => write!(f, "Y"),
        }
        .and_then(|()| match with_i {
            true => write!(f, "i"),
            false => Ok(()),
        })
    }
}

fn parse_suffix(s: &str) -> Result<(f64, Option<Suffix>)> {
    let with_i = s.ends_with("i");
    let mut iter = s.chars();
    if with_i {
        iter.next_back();
    }
    let suffix: Option<Suffix> = match iter.next_back() {
        Some('K') => Ok(Some((RawSuffix::K, with_i))),
        Some('M') => Ok(Some((RawSuffix::M, with_i))),
        Some('G') => Ok(Some((RawSuffix::G, with_i))),
        Some('T') => Ok(Some((RawSuffix::T, with_i))),
        Some('P') => Ok(Some((RawSuffix::P, with_i))),
        Some('E') => Ok(Some((RawSuffix::E, with_i))),
        Some('Z') => Ok(Some((RawSuffix::Z, with_i))),
        Some('Y') => Ok(Some((RawSuffix::Y, with_i))),
        Some('0'..='9') => Ok(None),
        _ => Err(format!("Failed to parse suffix '{}'", s)),
    }?;

    let suffix_len = match suffix {
        None => 0,
        Some((_, false)) => 1,
        Some((_, true)) => 2,
    };

    let number = s[..s.len() - suffix_len]
        .parse::<f64>()
        .map_err(|err| err.to_string())?;

    Ok((number, suffix))
}

fn parse_unit(s: String) -> Result<Unit> {
    match &s[..] {
        "auto" => Ok(Unit::Auto),
        "si" => Ok(Unit::Si),
        "iec" => Ok(Unit::Iec(false)),
        "iec-i" => Ok(Unit::Iec(true)),
        "none" => Ok(Unit::None),
        _ => Err("Unsupported unit is specified".to_owned()),
    }
}

struct TransformOptions {
    from: Transform,
    to: Transform,
}

struct Transform {
    unit: Unit,
}

type Field = (usize, usize);

fn parse_field(s: &String) -> Result<Field> {
    let field_max = std::usize::MAX.to_string();

    let re = Regex::new(r"^(\d{0,10})?(-(\d{0,10})?)?$").unwrap();
    match re.captures(s) {
        Some(caps) => {
            let first = caps.get(1).map(|m| m.as_str()).unwrap_or("1");
            // caps.get(2) refers to the second part of N-M format
            // If '-' is present in input then this match group will always exist
            // irrespective of whether M is provided or not.
            // To calculate the default value of M (when M is not provided)
            // we need to consider whether this match group exists or not.
            // If not then this means this is of the format 'N' which means that
            // the range should be N-N, hence the default is same as first.
            // If it exists then it means it can be of the following formats:
            // '-M', 'N-M', 'N-' or '-'
            // In the first two cases M is provided by user (default doesn't matter)
            // In the last two cases M is not provided and we should process all
            // fields till the end of line. Hence the default value is `field_max`
            let default_second = caps.get(2).map_or(first, |_| field_max.as_str());
            let second = caps.get(3).map(|m| m.as_str()).unwrap_or(default_second);
            Ok((
                first.parse::<usize>().map_err(|err| err.to_string())?,
                second.parse::<usize>().map_err(|err| err.to_string())?,
            ))
        }
        None => Err(format!("Invalid field format {}", s.to_string())),
    }
}

struct NumfmtOptions {
    transform: TransformOptions,
    padding: Option<isize>,
    header: usize,
    delimiter: Option<String>, // String to handle unicode. Only 1 char is allowed
    field: Field,
}

fn remove_suffix(i: f64, s: Option<Suffix>, u: &Unit, inp: &str) -> Result<f64> {
    match (s, u) {
        (None, _) => Ok(i),
        (Some((raw_suffix, false)), &Unit::Auto) | (Some((raw_suffix, false)), &Unit::Si) => {
            match raw_suffix {
                RawSuffix::K => Ok(i * 1e3),
                RawSuffix::M => Ok(i * 1e6),
                RawSuffix::G => Ok(i * 1e9),
                RawSuffix::T => Ok(i * 1e12),
                RawSuffix::P => Ok(i * 1e15),
                RawSuffix::E => Ok(i * 1e18),
                RawSuffix::Z => Ok(i * 1e21),
                RawSuffix::Y => Ok(i * 1e24),
            }
        }
        (Some((raw_suffix, false)), &Unit::Iec(false))
        | (Some((raw_suffix, true)), &Unit::Auto)
        | (Some((raw_suffix, true)), &Unit::Iec(true)) => match raw_suffix {
            RawSuffix::K => Ok(i * IEC_BASES[1]),
            RawSuffix::M => Ok(i * IEC_BASES[2]),
            RawSuffix::G => Ok(i * IEC_BASES[3]),
            RawSuffix::T => Ok(i * IEC_BASES[4]),
            RawSuffix::P => Ok(i * IEC_BASES[5]),
            RawSuffix::E => Ok(i * IEC_BASES[6]),
            RawSuffix::Z => Ok(i * IEC_BASES[7]),
            RawSuffix::Y => Ok(i * IEC_BASES[8]),
        },
        (Some(suffix), _) => Err(format!(
            "Rejecting suffix {} in input {}",
            DisplayableSuffix(suffix),
            inp
        )
        .to_owned()),
    }
}

fn transform_from(s: &str, opts: &Transform) -> Result<f64> {
    let (i, suffix) = parse_suffix(&s)?;
    remove_suffix(i, suffix, &opts.unit, &s).map(|n| n.round())
}

fn consider_suffix(i: f64, u: &Unit) -> Result<(f64, Option<Suffix>)> {
    let j = i.abs();
    match *u {
        Unit::Si => match j {
            _ if j < 1e3 => Ok((i, None)),
            _ if j < 1e6 => Ok((i / 1e3, Some((RawSuffix::K, false)))),
            _ if j < 1e9 => Ok((i / 1e6, Some((RawSuffix::M, false)))),
            _ if j < 1e12 => Ok((i / 1e9, Some((RawSuffix::G, false)))),
            _ if j < 1e15 => Ok((i / 1e12, Some((RawSuffix::T, false)))),
            _ if j < 1e18 => Ok((i / 1e15, Some((RawSuffix::P, false)))),
            _ if j < 1e21 => Ok((i / 1e18, Some((RawSuffix::E, false)))),
            _ if j < 1e24 => Ok((i / 1e21, Some((RawSuffix::Z, false)))),
            _ if j < 1e27 => Ok((i / 1e24, Some((RawSuffix::Y, false)))),
            _ => Err("Number is too big and unsupported".to_owned()),
        },
        Unit::Iec(with_i) => match j {
            _ if j < IEC_BASES[1] => Ok((i, None)),
            _ if j < IEC_BASES[2] => Ok((i / IEC_BASES[1], Some((RawSuffix::K, with_i)))),
            _ if j < IEC_BASES[3] => Ok((i / IEC_BASES[2], Some((RawSuffix::M, with_i)))),
            _ if j < IEC_BASES[4] => Ok((i / IEC_BASES[3], Some((RawSuffix::G, with_i)))),
            _ if j < IEC_BASES[5] => Ok((i / IEC_BASES[4], Some((RawSuffix::T, with_i)))),
            _ if j < IEC_BASES[6] => Ok((i / IEC_BASES[5], Some((RawSuffix::P, with_i)))),
            _ if j < IEC_BASES[7] => Ok((i / IEC_BASES[6], Some((RawSuffix::E, with_i)))),
            _ if j < IEC_BASES[8] => Ok((i / IEC_BASES[7], Some((RawSuffix::Z, with_i)))),
            _ if j < IEC_BASES[9] => Ok((i / IEC_BASES[8], Some((RawSuffix::Y, with_i)))),
            _ => Err("Number is too big and unsupported".to_owned()),
        },
        Unit::Auto => Err("Unit 'auto' isn't supported with --to options".to_owned()),
        Unit::None => Ok((i, None)),
    }
}

fn transform_to(s: f64, opts: &Transform) -> Result<String> {
    let (i2, s) = consider_suffix(s, &opts.unit)?;
    Ok(match s {
        None => format!("{}", i2),
        Some(s) => format!("{:.1}{}", i2, DisplayableSuffix(s)),
    })
}

fn format_column(index: usize, col: &str, options: &NumfmtOptions) -> Result<String> {
    let field = match options.delimiter {
        Some(_) => col,
        None => col.trim_start(),
    };
    let number = transform_to(
        transform_from(field, &options.transform.from)?,
        &options.transform.to,
    )?;

    Ok(match options.padding {
        None => match options.delimiter {
            None if index > 0 || col.starts_with(char::is_whitespace) => {
                format!("{:>padding$}", number, padding = col.len() as usize)
            }
            _ => number,
        },
        Some(p) if p == 0 => number,
        Some(p) if p > 0 => format!("{:>padding$}", number, padding = p as usize),
        Some(p) => format!("{:<padding$}", number, padding = p.abs() as usize),
    })
}

fn format_chosen_fields(index: usize, col: &str, options: &NumfmtOptions) -> Result<String> {
    if index + 1 >= options.field.0 && index + 1 <= options.field.1 {
        format_column(index, col, options)
    } else {
        Ok(col.to_string())
    }
}

fn format_string(source: String, options: &NumfmtOptions) -> Result<String> {
    struct SplitContext {
        last_char: char,
    };

    let mut ctx = SplitContext { last_char: '\0' };
    source
        .split(|c: char| match &options.delimiter {
            Some(s) => *s == c.to_string(),
            None => {
                let should_split = c.is_ascii_whitespace() && !ctx.last_char.is_ascii_whitespace();
                ctx.last_char = c;
                should_split
            }
        })
        .enumerate()
        .map(|(index, col)| format_chosen_fields(index, col, options))
        .collect::<Result<Vec<_>>>()
        .map(|x| x.join(&options.delimiter.as_ref().unwrap_or(&" ".to_owned())))
}

fn parse_options(args: &Matches) -> Result<NumfmtOptions> {
    let transform = TransformOptions {
        from: Transform {
            unit: args
                .opt_str("from")
                .map(parse_unit)
                .unwrap_or(Ok(Unit::None))?,
        },
        to: Transform {
            unit: args
                .opt_str("to")
                .map(parse_unit)
                .unwrap_or(Ok(Unit::None))?,
        },
    };

    let padding = match args.opt_str("padding") {
        Some(s) => s
            .parse::<isize>()
            .map(|p| Some(p))
            .map_err(|err| err.to_string()),
        None => Ok(None),
    }?;

    let header = match args.opt_default("header", "1") {
        Some(s) => s.parse::<usize>().map_err(|err| err.to_string()),
        None => Ok(0),
    }?;

    let delimiter = match args.opt_default("delimiter", " ") {
        Some(s) if s.len() == 1 => Ok(Some(s)),
        None => Ok(None),
        Some(_) => Err("Delimiter must be a single character".to_owned()),
    }?;

    let field = match args.opt_str("field") {
        Some(s) => parse_field(&s),
        None => Ok((1, 1)),
    }?;

    Ok(NumfmtOptions {
        transform: transform,
        padding: padding,
        header: header,
        delimiter: delimiter,
        field: field,
    })
}

fn handle_args(args: &Vec<String>, options: NumfmtOptions) -> Result<()> {
    for l in args {
        println!("{}", format_string(l.clone(), &options)?)
    }
    Ok(())
}

fn handle_stdin(options: NumfmtOptions) -> Result<()> {
    let stdin = std::io::stdin();
    let locked_stdin = stdin.lock();

    let mut lines = locked_stdin.lines();
    for l in lines.by_ref().take(options.header) {
        l.map(|s| println!("{}", s)).map_err(|e| e.to_string())?
    }

    for l in lines {
        l.map_err(|e| e.to_string()).and_then(|l| {
            let l = format_string(l, &options)?;
            Ok(println!("{}", l))
        })?
    }
    Ok(())
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");
    opts.optopt(
        "",
        "from",
        "auto-scale input numbers to UNITs; default is 'none'; see UNIT above",
        "UNIT",
    );
    opts.optopt(
        "",
        "to",
        "auto-scale output numbers to UNITs; see Unit above",
        "UNIT",
    );
    opts.optopt(
        "",
        "padding",
        "pad the output to N characters; positive N will right-align; negative N will left-align; padding is ignored if the output is wider than N",
        "N"
    );
    opts.optflagopt(
        "",
        "header",
        "print (without converting) the first N header lines; N defaults to 1 if not specified",
        "N",
    );
    opts.optflagopt(
        "d",
        "delimiter",
        "use X instead of white as field delimiter",
        "X",
    );
    opts.optflagopt(
        "",
        "field",
        "replace the numbers in these input fields (default=1)",
        "FIELDS",
    );

    let matches = opts.parse(&args[1..]).unwrap();
    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [STRING]... [OPTION]...", NAME);
        println!("");
        print!(
            "{}",
            opts.usage("Convert numbers from/to human-readable strings")
        );
        println!(
            "UNIT options:
   none   no auto-scaling is done; suffixes will trigger an error

   auto   accept optional single/two letter suffix:

		  1K = 1000, 1Ki = 1024, 1M = 1000000, 1Mi = 1048576,

   si     accept optional single letter suffix:

		  1K = 1000, 1M = 1000000, ...

   iec    accept optional single letter suffix:

		  1K = 1024, 1M = 1048576, ...

   iec-i  accept optional two-letter suffix:

		  1Ki = 1024, 1Mi = 1048576, ..."
        );

        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let options = parse_options(&matches).unwrap();

    if matches.free.len() == 0 {
        handle_stdin(options).unwrap()
    } else {
        handle_args(&matches.free, options).unwrap()
    };

    0
}
