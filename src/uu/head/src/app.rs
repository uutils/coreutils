use crate::constants;
use crate::parse;
use clap::{App, Arg};
use std::ffi::OsString;
use uucore::executable;

pub fn app<'a>() -> App<'a, 'a> {
    App::new(executable!())
        .version(constants::version())
        .about(constants::about())
        .usage(constants::usage())
        .arg(
            Arg::with_name(constants::bytes_name())
                .short("c")
                .long("bytes")
                .value_name("[-]NUM")
                .takes_value(true)
                .help(constants::bytes_help())
                .overrides_with_all(&[constants::bytes_name(), constants::lines_name()])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::with_name(constants::lines_name())
                .short("n")
                .long("lines")
                .value_name("[-]NUM")
                .takes_value(true)
                .help(constants::lines_help())
                .overrides_with_all(&[constants::lines_name(), constants::bytes_name()])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::with_name(constants::quiet_name())
                .short("q")
                .long("--quiet")
                .visible_alias("silent")
                .help(constants::quiet_help())
                .overrides_with_all(&[constants::verbose_name(), constants::quiet_name()]),
        )
        .arg(
            Arg::with_name(constants::verbose_name())
                .short("v")
                .long("verbose")
                .help(constants::verbose_help())
                .overrides_with_all(&[constants::quiet_name(), constants::verbose_name()]),
        )
        .arg(
            Arg::with_name(constants::zero_name())
                .short("z")
                .long("zero-terminated")
                .help(constants::zero_help())
                .overrides_with(constants::zero_name()),
        )
        .arg(Arg::with_name(constants::files_name()).multiple(true))
}
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Modes {
    Lines(usize),
    Bytes(usize),
}

pub fn parse_mode<F>(src: &str, closure: F) -> Result<(Modes, bool), String>
where
    F: FnOnce(usize) -> Modes,
{
    match parse::parse_num(src) {
        Ok((n, last)) => Ok((closure(n), last)),
        Err(reason) => match reason {
            parse::ParseError::Syntax => Err(format!("'{}'", src)),
            parse::ParseError::Overflow => {
                Err(format!("'{}': Value too large for defined datatype", src))
            }
        },
    }
}

//TODO: find better name for the `all_but_last` field
//and apply it across the code
#[derive(Debug)]
pub struct HeadOptions {
    pub quiet: bool,
    pub verbose: bool,
    pub zeroed: bool,
    pub all_but_last: bool,
    pub mode: Modes,
    pub files: Vec<String>,
}

fn arg_iterate<'a>(
    mut args: impl uucore::Args + 'a,
) -> Result<Box<dyn Iterator<Item = OsString> + 'a>, String> {
    if let Some(mut oss) = args.next() {
        if oss == "head" {
            if let Some(os) = args.next() {
                oss = os
            } else {
                return Ok(Box::new(vec![OsString::from("head")].into_iter()));
            };
        }
        if let Some(s) = oss.to_str() {
            match parse::parse_obsolete(s) {
                Some(Ok(iter)) => Ok(Box::new(
                    vec![OsString::from("head")]
                        .into_iter()
                        .chain(iter)
                        .chain(args),
                )),
                Some(Err(e)) => match e {
                    parse::ParseError::Syntax => Err(format!("bad argument format: '{}'", s)),
                    parse::ParseError::Overflow => Err(format!(
                        "invalid argument: '{}' Value too large for defined datatype",
                        s
                    )),
                },
                None => Ok(Box::new(
                    vec![OsString::from("head"), oss].into_iter().chain(args),
                )),
            }
        } else {
            Err("bad argument encoding".to_owned())
        }
    } else {
        Ok(Box::new(args))
    }
}

impl HeadOptions {
    pub fn new() -> HeadOptions {
        HeadOptions {
            quiet: false,
            verbose: false,
            zeroed: false,
            all_but_last: false,
            mode: Modes::Lines(10),
            files: Vec::new(),
        }
    }

    ///Construct options from matches
    pub fn get_from(args: impl uucore::Args) -> Result<Self, String> {
        let matches = app().get_matches_from(arg_iterate(args)?);

        let mut options = HeadOptions::new();

        options.quiet = matches.is_present(constants::quiet_name());
        options.verbose = matches.is_present(constants::verbose_name());
        options.zeroed = matches.is_present(constants::zero_name());

        let mode_and_from_end = if let Some(v) = matches.value_of(constants::bytes_name()) {
            match parse_mode(v, Modes::Bytes) {
                Ok(v) => v,
                Err(err) => {
                    return Err(format!("invalid number of bytes: {}", err));
                }
            }
        } else if let Some(v) = matches.value_of(constants::lines_name()) {
            match parse_mode(v, Modes::Lines) {
                Ok(v) => v,
                Err(err) => {
                    return Err(format!("invalid number of lines: {}", err));
                }
            }
        } else {
            (Modes::Lines(10), false)
        };

        options.mode = mode_and_from_end.0;
        options.all_but_last = mode_and_from_end.1;

        options.files = match matches.values_of(constants::files_name()) {
            Some(v) => v.map(|s| s.to_owned()).collect(),
            None => vec!["-".to_owned()],
        };
        //println!("{:#?}", options);
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    fn options(args: &str) -> Result<HeadOptions, String> {
        let combined = "head ".to_owned() + args;
        let args = combined.split_whitespace();
        HeadOptions::get_from(args.map(|s| OsString::from(s)))
    }
    #[test]
    fn test_args_modes() {
        let args = options("-n -10M -vz").unwrap();
        assert!(args.zeroed);
        assert!(args.verbose);
        assert!(args.all_but_last);
        assert_eq!(args.mode, Modes::Lines(10 * 1024 * 1024));
    }
    #[test]
    fn test_gnu_compatibility() {
        let args = options("-n 1 -c 1 -n 5 -c kiB -vqvqv").unwrap();
        assert!(args.mode == Modes::Bytes(1024));
        assert!(args.verbose);
        assert_eq!(options("-5").unwrap().mode, Modes::Lines(5));
        assert_eq!(options("-2b").unwrap().mode, Modes::Bytes(1024));
        assert_eq!(options("-5 -c 1").unwrap().mode, Modes::Bytes(1));
    }
    #[test]
    fn all_args_test() {
        assert!(options("--silent").unwrap().quiet);
        assert!(options("--quiet").unwrap().quiet);
        assert!(options("-q").unwrap().quiet);
        assert!(options("--verbose").unwrap().verbose);
        assert!(options("-v").unwrap().verbose);
        assert!(options("--zero-terminated").unwrap().zeroed);
        assert!(options("-z").unwrap().zeroed);
        assert_eq!(options("--lines 15").unwrap().mode, Modes::Lines(15));
        assert_eq!(options("-n 15").unwrap().mode, Modes::Lines(15));
        assert_eq!(options("--bytes 15").unwrap().mode, Modes::Bytes(15));
        assert_eq!(options("-c 15").unwrap().mode, Modes::Bytes(15));
    }
}
