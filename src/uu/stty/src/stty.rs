// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore clocal erange tcgetattr tcsetattr tcsanow tiocgwinsz tiocswinsz cfgetospeed cfsetospeed ushort vmin vtime

mod flags;

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use nix::libc::{c_ushort, O_NONBLOCK, TIOCGWINSZ, TIOCSWINSZ};
use nix::sys::termios::{
    cfgetospeed, cfsetospeed, tcgetattr, tcsetattr, ControlFlags, InputFlags, LocalFlags,
    OutputFlags, SpecialCharacterIndices, Termios,
};
use nix::{ioctl_read_bad, ioctl_write_ptr_bad};
use std::fs::File;
use std::io::{self, stdout, Stdout};
use std::ops::ControlFlow;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use uucore::error::{UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage};

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "ios",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
use flags::BAUD_RATES;
use flags::{CONTROL_CHARS, CONTROL_FLAGS, INPUT_FLAGS, LOCAL_FLAGS, OUTPUT_FLAGS};

const USAGE: &str = help_usage!("stty.md");
const SUMMARY: &str = help_about!("stty.md");

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
    file: Device,
    settings: Option<Vec<&'a str>>,
}

enum Device {
    File(File),
    Stdout(Stdout),
}

impl AsFd for Device {
    fn as_fd(&self) -> BorrowedFd<'_> {
        match self {
            Self::File(f) => f.as_fd(),
            Self::Stdout(stdout) => stdout.as_fd(),
        }
    }
}

impl AsRawFd for Device {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Self::File(f) => f.as_raw_fd(),
            Self::Stdout(stdout) => stdout.as_raw_fd(),
        }
    }
}

impl<'a> Options<'a> {
    fn from(matches: &'a ArgMatches) -> io::Result<Self> {
        Ok(Self {
            all: matches.get_flag(options::ALL),
            save: matches.get_flag(options::SAVE),
            file: match matches.get_one::<String>(options::FILE) {
                // Two notes here:
                // 1. O_NONBLOCK is needed because according to GNU docs, a
                //    POSIX tty can block waiting for carrier-detect if the
                //    "clocal" flag is not set. If your TTY is not connected
                //    to a modem, it is probably not relevant though.
                // 2. We never close the FD that we open here, but the OS
                //    will clean up the FD for us on exit, so it doesn't
                //    matter. The alternative would be to have an enum of
                //    BorrowedFd/OwnedFd to handle both cases.
                Some(f) => Device::File(
                    std::fs::OpenOptions::new()
                        .read(true)
                        .custom_flags(O_NONBLOCK)
                        .open(f)?,
                ),
                None => Device::Stdout(stdout()),
            },
            settings: matches
                .get_many::<String>(options::SETTINGS)
                .map(|v| v.map(|s| s.as_ref()).collect()),
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
    let mut termios = tcgetattr(opts.file.as_fd()).expect("Could not get terminal attributes");

    if let Some(settings) = &opts.settings {
        for setting in settings {
            if let ControlFlow::Break(false) = apply_setting(&mut termios, setting) {
                return Err(USimpleError::new(
                    1,
                    format!("invalid argument '{setting}'"),
                ));
            }
        }

        tcsetattr(
            opts.file.as_fd(),
            nix::sys::termios::SetArg::TCSANOW,
            &termios,
        )
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
    print!("speed {speed} baud; ");

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
            print!("speed {text} baud; ");
            break;
        }
    }

    if opts.all {
        let mut size = TermSize::default();
        unsafe { tiocgwinsz(opts.file.as_raw_fd(), &mut size as *mut _)? };
        print!("rows {}; columns {}; ", size.rows, size.columns);
    }

    #[cfg(any(target_os = "linux", target_os = "redox"))]
    {
        // For some reason the normal nix Termios struct does not expose the line,
        // so we get the underlying libc::termios struct to get that information.
        let libc_termios: nix::libc::termios = termios.clone().into();
        let line = libc_termios.c_line;
        print!("line = {line};");
    }

    println!();
    Ok(())
}

fn control_char_to_string(cc: nix::libc::cc_t) -> nix::Result<String> {
    if cc == 0 {
        return Ok("<undef>".to_string());
    }

    let (meta_prefix, code) = if cc >= 0x80 {
        ("M-", cc - 0x80)
    } else {
        ("", cc)
    };

    // Determine the '^'-prefix if applicable and character based on the code
    let (ctrl_prefix, character) = match code {
        // Control characters in ASCII range
        0..=0x1f => Ok(("^", (b'@' + code) as char)),
        // Printable ASCII characters
        0x20..=0x7e => Ok(("", code as char)),
        // DEL character
        0x7f => Ok(("^", '?')),
        // Out of range (above 8 bits)
        _ => Err(nix::errno::Errno::ERANGE),
    }?;

    Ok(format!("{meta_prefix}{ctrl_prefix}{character}"))
}

fn print_control_chars(termios: &Termios, opts: &Options) -> nix::Result<()> {
    if !opts.all {
        // TODO: this branch should print values that differ from defaults
        return Ok(());
    }

    for (text, cc_index) in CONTROL_CHARS {
        print!(
            "{text} = {}; ",
            control_char_to_string(termios.control_chars[*cc_index as usize])?
        );
    }
    println!(
        "min = {}; time = {};",
        termios.control_chars[SpecialCharacterIndices::VMIN as usize],
        termios.control_chars[SpecialCharacterIndices::VTIME as usize]
    );
    Ok(())
}

fn print_in_save_format(termios: &Termios) {
    print!(
        "{:x}:{:x}:{:x}:{:x}",
        termios.input_flags.bits(),
        termios.output_flags.bits(),
        termios.control_flags.bits(),
        termios.local_flags.bits()
    );
    for cc in termios.control_chars {
        print!(":{cc:x}");
    }
    println!();
}

fn print_settings(termios: &Termios, opts: &Options) -> nix::Result<()> {
    if opts.save {
        print_in_save_format(termios);
    } else {
        print_terminal_size(termios, opts)?;
        print_control_chars(termios, opts)?;
        print_flags(termios, opts, CONTROL_FLAGS);
        print_flags(termios, opts, INPUT_FLAGS);
        print_flags(termios, opts, OUTPUT_FLAGS);
        print_flags(termios, opts, LOCAL_FLAGS);
    }
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
    apply_baud_rate_flag(termios, s)?;

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

fn apply_baud_rate_flag(termios: &mut Termios, input: &str) -> ControlFlow<bool> {
    // BSDs use a u32 for the baud rate, so any decimal number applies.
    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    if let Ok(n) = input.parse::<u32>() {
        cfsetospeed(termios, n).expect("Failed to set baud rate");
        return ControlFlow::Break(true);
    }

    // Other platforms use an enum.
    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    for (text, baud_rate) in BAUD_RATES {
        if *text == input {
            cfsetospeed(termios, *baud_rate).expect("Failed to set baud rate");
            return ControlFlow::Break(true);
        }
    }
    ControlFlow::Continue(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(SUMMARY)
        .infer_long_args(true)
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("print all current settings in human-readable form")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SAVE)
                .short('g')
                .long(options::SAVE)
                .help("print all current settings in a stty-readable form")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .short('F')
                .long(options::FILE)
                .value_hint(clap::ValueHint::FilePath)
                .value_name("DEVICE")
                .help("open and use the specified DEVICE instead of stdin"),
        )
        .arg(
            Arg::new(options::SETTINGS)
                .action(ArgAction::Append)
                .help("settings to change"),
        )
}

impl TermiosFlag for ControlFlags {
    // TODO fix requires an MSRV of 1.82
    #[allow(clippy::unnecessary_map_or)]
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.control_flags.contains(*self)
            && group.map_or(true, |g| !termios.control_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.control_flags.set(*self, val);
    }
}

impl TermiosFlag for InputFlags {
    // TODO fix requires an MSRV of 1.82
    #[allow(clippy::unnecessary_map_or)]
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.input_flags.contains(*self)
            && group.map_or(true, |g| !termios.input_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.input_flags.set(*self, val);
    }
}

impl TermiosFlag for OutputFlags {
    // TODO fix requires an MSRV of 1.82
    #[allow(clippy::unnecessary_map_or)]
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.output_flags.contains(*self)
            && group.map_or(true, |g| !termios.output_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.output_flags.set(*self, val);
    }
}

impl TermiosFlag for LocalFlags {
    // TODO fix requires an MSRV of 1.82
    #[allow(clippy::unnecessary_map_or)]
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.local_flags.contains(*self)
            && group.map_or(true, |g| !termios.local_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.local_flags.set(*self, val);
    }
}
