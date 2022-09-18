// * This file is part of the uutils coreutils package.
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker:ignore tcgetattr tcsetattr tcsanow tiocgwinsz tiocswinsz cfgetospeed ushort

mod flags;

use clap::{crate_version, Arg, ArgMatches, Command};
use nix::libc::{c_ushort, TIOCGWINSZ, TIOCSWINSZ};
use nix::sys::termios::{
    cfgetospeed, tcgetattr, tcsetattr, ControlFlags, InputFlags, LocalFlags, OutputFlags, Termios,
};
use nix::{ioctl_read_bad, ioctl_write_ptr_bad};
use std::io::{self, stdout};
use std::ops::ControlFlow;
use std::os::unix::io::{AsRawFd, RawFd};
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "ios",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
use flags::BAUD_RATES;
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

impl<T> Flag<T> {
    pub const fn new(name: &'static str, flag: T) -> Self {
        Self {
            name,
            flag,
            show: true,
            sane: false,
            group: None,
        }
    }

    pub const fn new_grouped(name: &'static str, flag: T, group: T) -> Self {
        Self {
            name,
            flag,
            show: true,
            sane: false,
            group: Some(group),
        }
    }

    pub const fn hidden(mut self) -> Self {
        self.show = false;
        self
    }

    pub const fn sane(mut self) -> Self {
        self.sane = true;
        self
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
    save: bool,
    file: RawFd,
    settings: Option<Vec<&'a str>>,
}

impl<'a> Options<'a> {
    fn from(matches: &'a ArgMatches) -> io::Result<Self> {
        Ok(Self {
            all: matches.is_present(options::ALL),
            save: matches.is_present(options::SAVE),
            file: match matches.value_of(options::FILE) {
                Some(_f) => todo!(),
                None => stdout().as_raw_fd(),
            },
            settings: matches.values_of(options::SETTINGS).map(|v| v.collect()),
        })
    }
}

// Needs to be repr(C) because we pass it to the ioctl calls.
#[repr(C)]
#[derive(Default, Debug)]
pub struct TermSize {
    rows: c_ushort,
    columns: c_ushort,
    x: c_ushort,
    y: c_ushort,
}

ioctl_read_bad!(
    /// Get terminal window size
    tiocgwinsz,
    TIOCGWINSZ,
    TermSize
);

ioctl_write_ptr_bad!(
    /// Set terminal window size
    tiocswinsz,
    TIOCSWINSZ,
    TermSize
);

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_lossy();

    let matches = uu_app().try_get_matches_from(args)?;

    let opts = Options::from(&matches)?;

    stty(&opts)
}

fn stty(opts: &Options) -> UResult<()> {
    if opts.save && opts.all {
        return Err(USimpleError::new(
            1,
            "the options for verbose and stty-readable output styles are mutually exclusive",
        ));
    }

    if opts.settings.is_some() && (opts.save || opts.all) {
        return Err(USimpleError::new(
            1,
            "when specifying an output style, modes may not be set",
        ));
    }

    // TODO: Figure out the right error message for when tcgetattr fails
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
        print_settings(&termios, opts).expect("TODO: make proper error here from nix error");
    }
    Ok(())
}

fn print_terminal_size(termios: &Termios, opts: &Options) -> nix::Result<()> {
    let speed = cfgetospeed(termios);

    // BSDs use a u32 for the baud rate, so we can simply print it.
    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    print!("speed {} baud; ", speed);

    // Other platforms need to use the baud rate enum, so printing the right value
    // becomes slightly more complicated.
    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    for (text, baud_rate) in BAUD_RATES {
        if *baud_rate == speed {
            print!("speed {} baud; ", text);
            break;
        }
    }

    if opts.all {
        let mut size = TermSize::default();
        unsafe { tiocgwinsz(opts.file, &mut size as *mut _)? };
        print!("rows {}; columns {}; ", size.rows, size.columns);
    }

    #[cfg(any(target_os = "linux", target_os = "redox"))]
    {
        // For some reason the normal nix Termios struct does not expose the line,
        // so we get the underlying libc::termios struct to get that information.
        let libc_termios: nix::libc::termios = termios.clone().into();
        let line = libc_termios.c_line;
        print!("line = {};", line);
    }

    println!();
    Ok(())
}

fn print_settings(termios: &Termios, opts: &Options) -> nix::Result<()> {
    print_terminal_size(termios, opts)?;
    print_flags(termios, opts, CONTROL_FLAGS);
    print_flags(termios, opts, INPUT_FLAGS);
    print_flags(termios, opts, OUTPUT_FLAGS);
    print_flags(termios, opts, LOCAL_FLAGS);
    Ok(())
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
                print!("{} ", name);
                printed = true;
            }
        } else if opts.all || val != sane {
            if !val {
                print!("-");
            }
            print!("{} ", name);
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
    apply_flag(termios, CONTROL_FLAGS, name, remove)?;
    apply_flag(termios, INPUT_FLAGS, name, remove)?;
    apply_flag(termios, OUTPUT_FLAGS, name, remove)?;
    apply_flag(termios, LOCAL_FLAGS, name, remove)?;
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
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("print all current settings in human-readable form"),
        )
        .arg(
            Arg::new(options::SAVE)
                .short('g')
                .long(options::SAVE)
                .help("print all current settings in a stty-readable form"),
        )
        .arg(
            Arg::new(options::FILE)
                .short('F')
                .long(options::FILE)
                .takes_value(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_name("DEVICE")
                .help("open and use the specified DEVICE instead of stdin"),
        )
        .arg(
            Arg::new(options::SETTINGS)
                .takes_value(true)
                .multiple_values(true)
                .help("settings to change"),
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
