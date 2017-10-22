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

use getopts::{Matches, Options};
use std::io::BufRead;
use std::fmt;

static NAME: &'static str = "numfmt";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

type Result<T> = std::result::Result<T, String>;

enum TransformDirection {
    From,
    To,
}

enum Unit {
    Auto,
    Si,
    Iec,
    IecI,
}

enum Suffix {
    K,
    M,
    G,
    T,
    P,
    E,
    Ki,
    Mi,
    Gi,
    Ti,
    Pi,
    Ei,
}

impl fmt::Display for Suffix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Suffix::K => write!(f, "K"),
            Suffix::M => write!(f, "M"),
            Suffix::G => write!(f, "G"),
            Suffix::T => write!(f, "T"),
            Suffix::P => write!(f, "P"),
            Suffix::E => write!(f, "E"),
            Suffix::Ki => write!(f, "Ki"),
            Suffix::Mi => write!(f, "Mi"),
            Suffix::Gi => write!(f, "Gi"),
            Suffix::Ti => write!(f, "Ti"),
            Suffix::Pi => write!(f, "Pi"),
            Suffix::Ei => write!(f, "Ei"),
        }
    }
}

fn parse_suffix(s: String) -> Result<(f64, Option<Suffix>)> {
    let mut iter = s.chars();
    let (suffix, suffix_len) = match iter.next_back() {
        Some('K') => Ok((Some(Suffix::K), 1)),
        Some('M') => Ok((Some(Suffix::M), 1)),
        Some('G') => Ok((Some(Suffix::G), 1)),
        Some('T') => Ok((Some(Suffix::T), 1)),
        Some('P') => Ok((Some(Suffix::P), 1)),
        Some('E') => Ok((Some(Suffix::E), 1)),
        Some('i') => {
            match iter.next_back() {
                Some('K') => Ok((Some(Suffix::Ki), 2)),
                Some('M') => Ok((Some(Suffix::Mi), 2)),
                Some('G') => Ok((Some(Suffix::Gi), 2)),
                Some('T') => Ok((Some(Suffix::Ti), 2)),
                Some('P') => Ok((Some(Suffix::Pi), 2)),
                Some('E') => Ok((Some(Suffix::Ei), 2)),
                _ => Err("Failed to parse suffix"),
            }
        }
        _ => Ok((None, 0)),
    }?;

    let number = s[..s.len() - suffix_len].parse::<f64>().map_err(|err| {
        err.to_string()
    })?;

    Ok((number, suffix))
}

fn parse_unit(s: String) -> Result<Unit> {
    match &s[..] {
        "auto" => Ok(Unit::Auto),
        "si" => Ok(Unit::Si),
        "iec" => Ok(Unit::Iec),
        "iec-i" => Ok(Unit::IecI),
        _ => Err("Unsupported unit is specified".to_owned()),
    }
}

struct TransformOptions {
    direction: TransformDirection,
    unit: Unit,
}

struct NumfmtOptions {
    transform: TransformOptions,
    padding: isize,
    header: usize,
}

fn remove_suffix(i: f64, s: Option<Suffix>, u: &Unit) -> Result<f64> {
    match (s, u) {
        (None, _) => Ok(i),
        (Some(Suffix::K), &Unit::Auto) |
        (Some(Suffix::K), &Unit::Si) => Ok(i * 1000.),
        (Some(Suffix::M), &Unit::Auto) |
        (Some(Suffix::M), &Unit::Si) => Ok(i * 1000_000.),
        (Some(Suffix::G), &Unit::Auto) |
        (Some(Suffix::G), &Unit::Si) => Ok(i * 1000_000_000.),
        (Some(Suffix::T), &Unit::Auto) |
        (Some(Suffix::T), &Unit::Si) => Ok(i * 1000_000_000_000.),
        (Some(Suffix::P), &Unit::Auto) |
        (Some(Suffix::P), &Unit::Si) => Ok(i * 1000_000_000_000_000.),
        (Some(Suffix::E), &Unit::Auto) |
        (Some(Suffix::E), &Unit::Si) => Ok(i * 1000_000_000_000_000_000.),

        (Some(Suffix::Ki), &Unit::Auto) |
        (Some(Suffix::Ki), &Unit::IecI) |
        (Some(Suffix::K), &Unit::Iec) => Ok(i * 1024.),
        (Some(Suffix::Mi), &Unit::Auto) |
        (Some(Suffix::Mi), &Unit::IecI) |
        (Some(Suffix::M), &Unit::Iec) => Ok(i * 1048576.),
        (Some(Suffix::Gi), &Unit::Auto) |
        (Some(Suffix::Gi), &Unit::IecI) |
        (Some(Suffix::G), &Unit::Iec) => Ok(i * 1073741824.),
        (Some(Suffix::Ti), &Unit::Auto) |
        (Some(Suffix::Ti), &Unit::IecI) |
        (Some(Suffix::T), &Unit::Iec) => Ok(i * 1099511627776.),
        (Some(Suffix::Pi), &Unit::Auto) |
        (Some(Suffix::Pi), &Unit::IecI) |
        (Some(Suffix::P), &Unit::Iec) => Ok(i * 1125899906842624.),
        (Some(Suffix::Ei), &Unit::Auto) |
        (Some(Suffix::Ei), &Unit::IecI) |
        (Some(Suffix::E), &Unit::Iec) => Ok(i * 1152921504606846976.),

        (_, _) => Err("This suffix is unsupported for specified unit".to_owned()),
    }
}

fn transform_from(s: String, unit: &Unit) -> Result<String> {
    let (i, suffix) = parse_suffix(s)?;
    remove_suffix(i, suffix, unit).map(|n| n.round().to_string())
}

fn consider_suffix(i: f64, u: &Unit) -> Result<(f64, Option<Suffix>)> {
    match *u {
        Unit::Si => {
            match i {
                _ if i < 1000. => Ok((i, None)),
                _ if i < 1000_000. => Ok((i / 1000., Some(Suffix::K))),
                _ if i < 1000_000_000. => Ok((i / 1000_000., Some(Suffix::M))),
                _ if i < 1000_000_000_000. => Ok((i / 1000_000_000., Some(Suffix::G))),
                _ if i < 1000_000_000_000_000. => Ok((i / 1000_000_000_000., Some(Suffix::T))),
                _ if i < 1000_000_000_000_000_000. => Ok(
                    (i / 1000_000_000_000_000., Some(Suffix::P)),
                ),
                _ if i < 1000_000_000_000_000_000_000. => Ok((
                    i / 1000_000_000_000_000_000.,
                    Some(Suffix::E),
                )),
                _ => Err("Number is too big and unsupported".to_owned()),
            }
        }
        Unit::Iec => {
            match i {
                _ if i < 1024. => Ok((i, None)),
                _ if i < 1048576. => Ok((i / 1024., Some(Suffix::K))),
                _ if i < 1073741824. => Ok((i / 1048576., Some(Suffix::M))),
                _ if i < 1099511627776. => Ok((i / 1073741824., Some(Suffix::G))),
                _ if i < 1125899906842624. => Ok((i / 1099511627776., Some(Suffix::T))),
                _ if i < 1152921504606846976. => Ok((i / 1125899906842624., Some(Suffix::P))),
                _ if i < 1180591620717411303424. => Ok((i / 1152921504606846976., Some(Suffix::E))),
                _ => Err("Number is too big and unsupported".to_owned()),
            }
        }
        Unit::IecI => {
            match i {
                _ if i < 1024. => Ok((i, None)),
                _ if i < 1048576. => Ok((i / 1024., Some(Suffix::Ki))),
                _ if i < 1073741824. => Ok((i / 1048576., Some(Suffix::Mi))),
                _ if i < 1099511627776. => Ok((i / 1073741824., Some(Suffix::Gi))),
                _ if i < 1125899906842624. => Ok((i / 1099511627776., Some(Suffix::Ti))),
                _ if i < 1152921504606846976. => Ok((i / 1125899906842624., Some(Suffix::Pi))),
                _ if i < 1180591620717411303424. => Ok(
                    (i / 1152921504606846976., Some(Suffix::Ei)),
                ),
                _ => Err("Number is too big and unsupported".to_owned()),
            }
        }
        Unit::Auto => Err("Unit 'auto' isn't supported with --to options".to_owned()),
    }
}

fn transform_to(s: String, unit: &Unit) -> Result<String> {
    let i = s.parse::<f64>().map_err(|err| err.to_string())?;
    let (i2, s) = consider_suffix(i, unit)?;
    Ok(match s {
        None => format!("{}", i2),
        Some(s) => format!("{:.1}{}", i2, s),
    })
}

fn format_string(source: String, options: &NumfmtOptions) -> Result<String> {
    let number = match options.transform.direction {
        TransformDirection::From => transform_from(source, &options.transform.unit)?,
        TransformDirection::To => transform_to(source, &options.transform.unit)?,
    };

    Ok(match options.padding {
        p if p == 0 => number,
        p if p > 0 => format!("{:>padding$}", number, padding = p as usize),
        p => format!("{:<padding$}", number, padding = p.abs() as usize),
    })
}

fn parse_options(args: &Matches) -> Result<NumfmtOptions> {
    let transform = if args.opt_present("from") {
        TransformOptions {
            direction: TransformDirection::From,
            unit: parse_unit(args.opt_str("from").ok_or("'--from' should have argument")?)?,
        }
    } else if args.opt_present("to") {
        TransformOptions {
            direction: TransformDirection::To,
            unit: parse_unit(args.opt_str("to").ok_or("'--to' should have argument")?)?,
        }
    } else {
        return Err("Either '--from' or '--to' should be specified".to_owned());
    };

    let padding = match args.opt_str("padding") {
        Some(s) => s.parse::<isize>().map_err(|err| err.to_string()),
        None => Ok(0),
    }?;

    let header = match args.opt_default("header", "1") {
        Some(s) => s.parse::<usize>().map_err(|err| err.to_string()),
        None => Ok(0),
    }?;

    Ok(NumfmtOptions {
        transform: transform,
        padding: padding,
        header: header,
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
