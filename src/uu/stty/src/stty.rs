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
use std::os::unix::io::{AsRawFd, RawFd};
use uucore::error::UResult;
use uucore::{format_usage, InvalidEncodingHandling};

use flags::{CONTROL_FLAGS, INPUT_FLAGS, LOCAL_FLAGS, OUTPUT_FLAGS};

const NAME: &str = "stty";
const USAGE: &str = "\
{} [-F DEVICE | --file=DEVICE] [SETTING]...
{} [-F DEVICE | --file=DEVICE] [-a|--all]
{} [-F DEVICE | --file=DEVICE] [-g|--save]";
const SUMMARY: &str = "Print or change terminal characteristics.";

pub struct Flag<T> {
    name: &'static str,
    flag: T,
    show: bool,
    sane: bool,
}

trait TermiosFlag {
    fn is_in(&self, termios: &Termios) -> bool;
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
            apply_setting(&mut termios, setting);
        }

        tcsetattr(opts.file, nix::sys::termios::SetArg::TCSANOW, &termios)
            .expect("Could not write terminal attributes");
    } else {
        print_settings(&termios, opts);
    }
    Ok(())
}

fn print_settings(termios: &Termios, opts: &Options) {
    if print_flags(termios, opts, &CONTROL_FLAGS) {
        println!();
    }
    if print_flags(termios, opts, &INPUT_FLAGS) {
        println!();
    }
    if print_flags(termios, opts, &OUTPUT_FLAGS) {
        println!();
    }
    if print_flags(termios, opts, &LOCAL_FLAGS) {
        println!();
    }
}

fn print_flags<T>(termios: &Termios, opts: &Options, flags: &[Flag<T>]) -> bool
where
    Flag<T>: TermiosFlag,
{
    let mut printed = false;
    for flag in flags {
        if !flag.show {
            continue;
        }
        let val = flag.is_in(termios);
        if opts.all || val != flag.sane {
            if !val {
                print!("-");
            }
            print!("{} ", flag.name);
            printed = true;
        }
    }
    printed
}

fn apply_setting(termios: &mut Termios, s: &str) -> Option<()> {
    if let Some(()) = apply_flag(termios, &CONTROL_FLAGS, s) {
        return Some(());
    }
    if let Some(()) = apply_flag(termios, &INPUT_FLAGS, s) {
        return Some(());
    }
    if let Some(()) = apply_flag(termios, &OUTPUT_FLAGS, s) {
        return Some(());
    }
    if let Some(()) = apply_flag(termios, &LOCAL_FLAGS, s) {
        return Some(());
    }
    None
}

fn apply_flag<T>(termios: &mut Termios, flags: &[Flag<T>], name: &str) -> Option<()>
where
    T: Copy,
    Flag<T>: TermiosFlag,
{
    let (remove, name) = strip_hyphen(name);
    find(flags, name)?.apply(termios, !remove);
    Some(())
}

fn strip_hyphen(s: &str) -> (bool, &str) {
    match s.strip_prefix('-') {
        Some(s) => (true, s),
        None => (false, s),
    }
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

impl TermiosFlag for Flag<ControlFlags> {
    fn is_in(&self, termios: &Termios) -> bool {
        termios.control_flags.contains(self.flag)
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.control_flags.set(self.flag, val)
    }
}

impl TermiosFlag for Flag<InputFlags> {
    fn is_in(&self, termios: &Termios) -> bool {
        termios.input_flags.contains(self.flag)
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.input_flags.set(self.flag, val)
    }
}

impl TermiosFlag for Flag<OutputFlags> {
    fn is_in(&self, termios: &Termios) -> bool {
        termios.output_flags.contains(self.flag)
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.output_flags.set(self.flag, val)
    }
}

impl TermiosFlag for Flag<LocalFlags> {
    fn is_in(&self, termios: &Termios) -> bool {
        termios.local_flags.contains(self.flag)
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.local_flags.set(self.flag, val)
    }
}

fn find<'a, T>(flags: &'a [Flag<T>], flag_name: &str) -> Option<&'a Flag<T>>
where
    T: Copy,
{
    flags.iter().find_map(|flag| {
        if flag.name == flag_name {
            Some(flag)
        } else {
            None
        }
    })
}
