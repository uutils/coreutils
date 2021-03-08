//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Yury Krivopalov <ykrivopalov@yandex.ru>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{App, AppSettings, Arg, ArgMatches};
use std::fmt;
use std::io::{BufRead, Write};
use uucore::ranges::Range;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Convert numbers from/to human-readable strings";
static LONG_HELP: &str = "UNIT options:
   none   no auto-scaling is done; suffixes will trigger an error

   auto   accept optional single/two letter suffix:

          1K = 1000, 1Ki = 1024, 1M = 1000000, 1Mi = 1048576,

   si     accept optional single letter suffix:

          1K = 1000, 1M = 1000000, ...

   iec    accept optional single letter suffix:

          1K = 1024, 1M = 1048576, ...

   iec-i  accept optional two-letter suffix:

          1Ki = 1024, 1Mi = 1048576, ...

FIELDS supports cut(1) style field ranges:
  N    N'th field, counted from 1
  N-   from N'th field, to end of line
  N-M  from N'th to M'th field (inclusive)
  -M   from first to M'th field (inclusive)
  -    all fields
Multiple fields/ranges can be separated with commas
";

mod options {
    pub const FIELD: &str = "field";
    pub const FIELD_DEFAULT: &str = "1";
    pub const FROM: &str = "from";
    pub const FROM_DEFAULT: &str = "none";
    pub const HEADER: &str = "header";
    pub const HEADER_DEFAULT: &str = "1";
    pub const NUMBER: &str = "NUMBER";
    pub const PADDING: &str = "padding";
    pub const TO: &str = "to";
    pub const TO_DEFAULT: &str = "none";
}

fn get_usage() -> String {
    format!("{0} [OPTION]... [NUMBER]...", executable!())
}

const SI_BASES: [f64; 10] = [1., 1e3, 1e6, 1e9, 1e12, 1e15, 1e18, 1e21, 1e24, 1e27];

const IEC_BASES: [f64; 10] = [
    1.,
    1_024.,
    1_048_576.,
    1_073_741_824.,
    1_099_511_627_776.,
    1_125_899_906_842_624.,
    1_152_921_504_606_846_976.,
    1_180_591_620_717_411_303_424.,
    1_208_925_819_614_629_174_706_176.,
    1_237_940_039_285_380_274_899_124_224.,
];

type Result<T> = std::result::Result<T, String>;

type WithI = bool;

enum Unit {
    Auto,
    Si,
    Iec(WithI),
    None,
}

#[derive(Clone, Copy, Debug)]
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
    if s.is_empty() {
        return Err("invalid number: ‘’".to_string());
    }

    let with_i = s.ends_with('i');
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
        _ => Err(format!("invalid suffix in input: ‘{}’", s)),
    }?;

    let suffix_len = match suffix {
        None => 0,
        Some((_, false)) => 1,
        Some((_, true)) => 2,
    };

    let number = s[..s.len() - suffix_len]
        .parse::<f64>()
        .map_err(|_| format!("invalid number: ‘{}’", s))?;

    Ok((number, suffix))
}

fn parse_unit(s: &str) -> Result<Unit> {
    match s {
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

struct NumfmtOptions {
    transform: TransformOptions,
    padding: isize,
    header: usize,
    fields: Vec<Range>,
}

/// Iterate over a line's fields, where each field is a contiguous sequence of
/// non-whitespace, optionally prefixed with one or more characters of leading
/// whitespace. Fields are returned as tuples of `(prefix, field)`.
///
/// # Examples:
///
/// ```
/// let mut fields = uu_numfmt::WhitespaceSplitter { s: Some("    1234 5") };
///
/// assert_eq!(Some(("    ", "1234")), fields.next());
/// assert_eq!(Some((" ", "5")), fields.next());
/// assert_eq!(None, fields.next());
/// ```
///
/// Delimiters are included in the results; `prefix` will be empty only for
/// the first field of the line (including the case where the input line is
/// empty):
///
/// ```
/// let mut fields = uu_numfmt::WhitespaceSplitter { s: Some("first second") };
///
/// assert_eq!(Some(("", "first")), fields.next());
/// assert_eq!(Some((" ", "second")), fields.next());
///
/// let mut fields = uu_numfmt::WhitespaceSplitter { s: Some("") };
///
/// assert_eq!(Some(("", "")), fields.next());
/// ```
pub struct WhitespaceSplitter<'a> {
    pub s: Option<&'a str>,
}

impl<'a> Iterator for WhitespaceSplitter<'a> {
    type Item = (&'a str, &'a str);

    /// Yield the next field in the input string as a tuple `(prefix, field)`.
    fn next(&mut self) -> Option<Self::Item> {
        let haystack = self.s?;

        let (prefix, field) = haystack.split_at(
            haystack
                .find(|c: char| !c.is_whitespace())
                .unwrap_or_else(|| haystack.len()),
        );

        let (field, rest) = field.split_at(
            field
                .find(|c: char| c.is_whitespace())
                .unwrap_or_else(|| field.len()),
        );

        self.s = if !rest.is_empty() { Some(rest) } else { None };

        Some((prefix, field))
    }
}

fn remove_suffix(i: f64, s: Option<Suffix>, u: &Unit) -> Result<f64> {
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
        (_, _) => Err("This suffix is unsupported for specified unit".to_owned()),
    }
}

fn transform_from(s: &str, opts: &Transform) -> Result<f64> {
    let (i, suffix) = parse_suffix(s)?;

    remove_suffix(i, suffix, &opts.unit).map(|n| if n < 0.0 { -n.abs().ceil() } else { n.ceil() })
}

/// Divide numerator by denominator, with ceiling.
///
/// If the result of the division is less than 10.0, truncate the result
/// to the next highest tenth.
///
/// Otherwise, truncate the result to the next highest whole number.
///
/// # Examples:
///
/// ```
/// use uu_numfmt::div_ceil;
///
/// assert_eq!(div_ceil(1.01, 1.0), 1.1);
/// assert_eq!(div_ceil(999.1, 1000.), 1.0);
/// assert_eq!(div_ceil(1001., 10.), 101.);
/// assert_eq!(div_ceil(9991., 10.), 1000.);
/// assert_eq!(div_ceil(-12.34, 1.0), -13.0);
/// assert_eq!(div_ceil(1000.0, -3.14), -319.0);
/// assert_eq!(div_ceil(-271828.0, -271.0), 1004.0);
/// ```
pub fn div_ceil(n: f64, d: f64) -> f64 {
    let v = n / (d / 10.0);
    let (v, sign) = if v < 0.0 { (v.abs(), -1.0) } else { (v, 1.0) };

    if v < 100.0 {
        v.ceil() / 10.0 * sign
    } else {
        (v / 10.0).ceil() * sign
    }
}

fn consider_suffix(n: f64, u: &Unit) -> Result<(f64, Option<Suffix>)> {
    use RawSuffix::*;

    let abs_n = n.abs();
    let suffixes = [K, M, G, T, P, E, Z, Y];

    let (bases, with_i) = match *u {
        Unit::Si => (&SI_BASES, false),
        Unit::Iec(with_i) => (&IEC_BASES, with_i),
        Unit::Auto => return Err("Unit 'auto' isn't supported with --to options".to_owned()),
        Unit::None => return Ok((n, None)),
    };

    let i = match abs_n {
        _ if abs_n <= bases[1] - 1.0 => return Ok((n, None)),
        _ if abs_n < bases[2] => 1,
        _ if abs_n < bases[3] => 2,
        _ if abs_n < bases[4] => 3,
        _ if abs_n < bases[5] => 4,
        _ if abs_n < bases[6] => 5,
        _ if abs_n < bases[7] => 6,
        _ if abs_n < bases[8] => 7,
        _ if abs_n < bases[9] => 8,
        _ => return Err("Number is too big and unsupported".to_string()),
    };

    let v = div_ceil(n, bases[i]);

    // check if rounding pushed us into the next base
    if v.abs() >= bases[1] {
        Ok((v / bases[1], Some((suffixes[i], with_i))))
    } else {
        Ok((v, Some((suffixes[i - 1], with_i))))
    }
}

fn transform_to(s: f64, opts: &Transform) -> Result<String> {
    let (i2, s) = consider_suffix(s, &opts.unit)?;
    Ok(match s {
        None => format!("{}", i2),
        Some(s) if i2.abs() < 10.0 => format!("{:.1}{}", i2, DisplayableSuffix(s)),
        Some(s) => format!("{:.0}{}", i2, DisplayableSuffix(s)),
    })
}

fn format_string(
    source: &str,
    options: &NumfmtOptions,
    implicit_padding: Option<isize>,
) -> Result<String> {
    let number = transform_to(
        transform_from(source, &options.transform.from)?,
        &options.transform.to,
    )?;

    Ok(match implicit_padding.unwrap_or(options.padding) {
        p if p == 0 => number,
        p if p > 0 => format!("{:>padding$}", number, padding = p as usize),
        p => format!("{:<padding$}", number, padding = p.abs() as usize),
    })
}

fn format_and_print(s: &str, options: &NumfmtOptions) -> Result<()> {
    for (n, (prefix, field)) in (1..).zip(WhitespaceSplitter { s: Some(s) }) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        if field_selected {
            let empty_prefix = prefix.is_empty();

            // print delimiter before second and subsequent fields
            let prefix = if n > 1 {
                print!(" ");
                &prefix[1..]
            } else {
                &prefix
            };

            let implicit_padding = if !empty_prefix && options.padding == 0 {
                Some((prefix.len() + field.len()) as isize)
            } else {
                None
            };

            print!("{}", format_string(&field, options, implicit_padding)?);
        } else {
            // print unselected field without conversion
            print!("{}{}", prefix, field);
        }
    }

    println!();

    Ok(())
}

fn parse_options(args: &ArgMatches) -> Result<NumfmtOptions> {
    let from = parse_unit(args.value_of(options::FROM).unwrap())?;
    let to = parse_unit(args.value_of(options::TO).unwrap())?;

    let transform = TransformOptions {
        from: Transform { unit: from },
        to: Transform { unit: to },
    };

    let padding = match args.value_of(options::PADDING) {
        Some(s) => s.parse::<isize>().map_err(|err| err.to_string()),
        None => Ok(0),
    }?;

    let header = match args.occurrences_of(options::HEADER) {
        0 => Ok(0),
        _ => {
            let value = args.value_of(options::HEADER).unwrap();

            value
                .parse::<usize>()
                .map_err(|_| value)
                .and_then(|n| match n {
                    0 => Err(value),
                    _ => Ok(n),
                })
                .map_err(|value| format!("invalid header value ‘{}’", value))
        }
    }?;

    let fields = match args.value_of(options::FIELD) {
        Some("-") => vec![Range {
            low: 1,
            high: std::usize::MAX,
        }],
        Some(v) => Range::from_list(v)?,
        None => unreachable!(),
    };

    Ok(NumfmtOptions {
        transform,
        padding,
        header,
        fields,
    })
}

fn handle_args<'a>(args: impl Iterator<Item = &'a str>, options: NumfmtOptions) -> Result<()> {
    for l in args {
        format_and_print(l, &options)?;
    }

    Ok(())
}

fn handle_stdin(options: NumfmtOptions) -> Result<()> {
    let stdin = std::io::stdin();
    let locked_stdin = stdin.lock();

    let mut lines = locked_stdin.lines();
    for l in lines.by_ref().take(options.header) {
        l.map(|s| println!("{}", s)).map_err(|e| e.to_string())?;
    }

    for l in lines {
        l.map_err(|e| e.to_string())
            .and_then(|l| format_and_print(&l, &options))?;
    }

    Ok(())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(LONG_HELP)
        .setting(AppSettings::AllowNegativeNumbers)
        .arg(
            Arg::with_name(options::FIELD)
                .long(options::FIELD)
                .help("replace the numbers in these input fields (default=1) see FIELDS below")
                .value_name("FIELDS")
                .default_value(options::FIELD_DEFAULT),
        )
        .arg(
            Arg::with_name(options::FROM)
                .long(options::FROM)
                .help("auto-scale input numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::FROM_DEFAULT),
        )
        .arg(
            Arg::with_name(options::TO)
                .long(options::TO)
                .help("auto-scale output numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::TO_DEFAULT),
        )
        .arg(
            Arg::with_name(options::PADDING)
                .long(options::PADDING)
                .help(
                    "pad the output to N characters; positive N will \
                    right-align; negative N will left-align; padding is \
                    ignored if the output is wider than N; the default is \
                    to automatically pad if a whitespace is found",
                )
                .value_name("N"),
        )
        .arg(
            Arg::with_name(options::HEADER)
                .long(options::HEADER)
                .help(
                    "print (without converting) the first N header lines; \
                    N defaults to 1 if not specified",
                )
                .value_name("N")
                .default_value(options::HEADER_DEFAULT)
                .hide_default_value(true),
        )
        .arg(Arg::with_name(options::NUMBER).hidden(true).multiple(true))
        .get_matches_from(args);

    let result =
        parse_options(&matches).and_then(|options| match matches.values_of(options::NUMBER) {
            Some(values) => handle_args(values, options),
            None => handle_stdin(options),
        });

    match result {
        Err(e) => {
            std::io::stdout().flush().expect("error flushing stdout");
            show_info!("{}", e);
            1
        }
        _ => 0,
    }
}
