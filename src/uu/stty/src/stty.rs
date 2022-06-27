// * This file is part of the uutils coreutils package.
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker:ignore tcgetattr tcsetattr tcsanow

mod flags;

use clap::{crate_version, Arg, ArgMatches, Command};
use nix::sys::termios::{
    tcgetattr, tcsetattr, ControlFlags, InputFlags, LocalFlags, OutputFlags, Termios,
};
use std::io::{self, stdout};
use std::ops::ControlFlow;
use std::os::unix::io::{AsRawFd, RawFd};
use uucore::error::{UResult, USimpleError};
use uucore::{format_usage, InvalidEncodingHandling};

use flags::{CONTROL_FLAGS, INPUT_FLAGS, LOCAL_FLAGS, OUTPUT_FLAGS};

const NAME: &str = "stty";
const USAGE: &str = "\
{} [-F DEVICE | --file=DEVICE] [SETTING]...
{} [-F DEVICE | --file=DEVICE] [-a|--all]
{} [-F DEVICE | --file=DEVICE] [-g|--save]";
const SUMMARY: &str = "Print or change terminal characteristics.";

#[derive(Clone, Copy, Debug)]
pub struct Flag<T> {
    name: &'static str,
    flag: T,
    show: bool,
    sane: bool,
    group: Option<T>,
}

impl<T> Flag<T>
where
    T: Copy,
{
    pub const fn new(name: &'static str, flag: T) -> Self {
        Self {
            name,
            flag,
            show: true,
            sane: false,
            group: None,
        }
    }

    pub const fn hidden(&self) -> Self {
        Self {
            show: false,
            ..*self
        }
    }

    pub const fn sane(&self) -> Self {
        Self {
            sane: true,
            ..*self
        }
    }

    pub const fn group(&self, group: T) -> Self {
        Self {
            group: Some(group),
            ..*self
        }
    }
}

trait TermiosFlag: Copy {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool;
    fn apply(&self, termios: &mut Termios, val: bool);
}

mod options {
    pub const ALL: &str = "all";
    pub const SAVE: &str = "save";
    pub const FILE: &str = "file";
    pub const SETTINGS: &str = "settings";
}

struct Options<'a> {
    all: bool,
    _save: bool,
    file: RawFd,
    settings: Option<Vec<&'a str>>,
}

impl<'a> Options<'a> {
    fn from(matches: &'a ArgMatches) -> io::Result<Self> {
        Ok(Self {
            all: matches.is_present(options::ALL),
            _save: matches.is_present(options::SAVE),
            file: match matches.value_of(options::FILE) {
                Some(_f) => todo!(),
                None => stdout().as_raw_fd(),
            },
            settings: matches.values_of(options::SETTINGS).map(|v| v.collect()),
        })
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let opts = Options::from(&matches)?;

    stty(&opts)
}

fn stty(opts: &Options) -> UResult<()> {
    // TODO: Figure out the right error message
    let mut termios = tcgetattr(opts.file).expect("Could not get terminal attributes");
    if let Some(settings) = &opts.settings {
        for setting in settings {
            if let ControlFlow::Break(false) = apply_setting(&mut termios, setting) {
                return Err(USimpleError::new(
                    1,
                    format!("invalid argument '{}'", setting),
                ));
            }
        }

        tcsetattr(opts.file, nix::sys::termios::SetArg::TCSANOW, &termios)
            .expect("Could not write terminal attributes");
    } else {
        print_settings(&termios, opts);
    }
    Ok(())
}

fn print_settings(termios: &Termios, opts: &Options) {
    print_flags(termios, opts, &CONTROL_FLAGS);
    print_flags(termios, opts, &INPUT_FLAGS);
    print_flags(termios, opts, &OUTPUT_FLAGS);
    print_flags(termios, opts, &LOCAL_FLAGS);
}

fn print_flags<T: TermiosFlag>(termios: &Termios, opts: &Options, flags: &[Flag<T>]) {
    let mut printed = false;
    for &Flag {
        name,
        flag,
        show,
        sane,
        group,
    } in flags
    {
        if !show {
            continue;
        }
        let val = flag.is_in(termios, group);
        if group.is_some() {
            if val && (!sane || opts.all) {
                print!("{name} ");
                printed = true;
            }
        } else if opts.all || val != sane {
            if !val {
                print!("-");
            }
            print!("{name} ");
            printed = true;
        }
    }
    if printed {
        println!();
    }
}

/// Apply a single setting
///
/// The value inside the `Break` variant of the `ControlFlow` indicates whether
/// the setting has been applied.
fn apply_setting(termios: &mut Termios, s: &str) -> ControlFlow<bool> {
    let (remove, name) = match s.strip_prefix('-') {
        Some(s) => (true, s),
        None => (false, s),
    };
    apply_flag(termios, &CONTROL_FLAGS, name, remove)?;
    apply_flag(termios, &INPUT_FLAGS, name, remove)?;
    apply_flag(termios, &OUTPUT_FLAGS, name, remove)?;
    apply_flag(termios, &LOCAL_FLAGS, name, remove)?;
    ControlFlow::Break(false)
}

/// Apply a flag to a slice of flags
///
/// The value inside the `Break` variant of the `ControlFlow` indicates whether
/// the setting has been applied.
fn apply_flag<T: TermiosFlag>(
    termios: &mut Termios,
    flags: &[Flag<T>],
    input: &str,
    remove: bool,
) -> ControlFlow<bool> {
    for Flag {
        name, flag, group, ..
    } in flags
    {
        if input == *name {
            // Flags with groups cannot be removed
            // Since the name matches, we can short circuit and don't have to check the other flags.
            if remove && group.is_some() {
                return ControlFlow::Break(false);
            }
            // If there is a group, the bits for that group should be cleared before applying the flag
            if let Some(group) = group {
                group.apply(termios, false);
            }
            flag.apply(termios, !remove);
            return ControlFlow::Break(true);
        }
    }
    ControlFlow::Continue(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(SUMMARY)
        .infer_long_args(true)
        .arg(Arg::new(options::ALL).short('a').long(options::ALL))
        .arg(Arg::new(options::SAVE).short('g').long(options::SAVE))
        .arg(
            Arg::new(options::FILE)
                .short('F')
                .long(options::FILE)
                .takes_value(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::SETTINGS)
                .takes_value(true)
                .multiple_values(true),
        )
}

impl TermiosFlag for ControlFlags {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.control_flags.contains(*self)
            && group.map_or(true, |g| !termios.control_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.control_flags.set(*self, val);
    }
}

impl TermiosFlag for InputFlags {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.input_flags.contains(*self)
            && group.map_or(true, |g| !termios.input_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.input_flags.set(*self, val);
    }
}

impl TermiosFlag for OutputFlags {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.output_flags.contains(*self)
            && group.map_or(true, |g| !termios.output_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.output_flags.set(*self, val);
    }
}

impl TermiosFlag for LocalFlags {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.local_flags.contains(*self)
            && group.map_or(true, |g| !termios.local_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.local_flags.set(*self, val);
    }
}
