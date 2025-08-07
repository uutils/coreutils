// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore clocal erange tcgetattr tcsetattr tcsanow tiocgwinsz tiocswinsz cfgetospeed cfsetospeed ushort vmin vtime cflag lflag ispeed ospeed
// spell-checker:ignore parenb parodd cmspar hupcl cstopb cread clocal crtscts CSIZE
// spell-checker:ignore ignbrk brkint ignpar parmrk inpck istrip inlcr igncr icrnl ixoff ixon iuclc ixany imaxbel iutf
// spell-checker:ignore opost olcuc ocrnl onlcr onocr onlret ofdel nldly crdly tabdly bsdly vtdly ffdly ofill
// spell-checker:ignore isig icanon iexten echoe crterase echok echonl noflsh xcase tostop echoprt prterase echoctl ctlecho echoke crtkill flusho extproc
// spell-checker:ignore lnext rprnt susp swtch vdiscard veof veol verase vintr vkill vlnext vquit vreprint vstart vstop vsusp vswtc vwerase werase
// spell-checker:ignore sigquit sigtstp
// spell-checker:ignore cbreak decctlq evenp litout oddp tcsadrain

mod flags;

use crate::flags::AllFlags;
use crate::flags::COMBINATION_SETTINGS;
use clap::{Arg, ArgAction, ArgMatches, Command};
use nix::libc::{O_NONBLOCK, TIOCGWINSZ, TIOCSWINSZ, c_ushort};
use nix::sys::termios::{
    ControlFlags, InputFlags, LocalFlags, OutputFlags, SetArg, SpecialCharacterIndices as S,
    Termios, cfgetospeed, cfsetospeed, tcgetattr, tcsetattr,
};
use nix::{ioctl_read_bad, ioctl_write_ptr_bad};
use std::fs::File;
use std::io::{self, Stdout, stdout};
use std::num::IntErrorKind;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use uucore::error::{UError, UResult, USimpleError};
use uucore::format_usage;
use uucore::translate;

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

const ASCII_DEL: u8 = 127;

// Sane defaults for control characters.
const SANE_CONTROL_CHARS: [(S, u8); 12] = [
    (S::VINTR, 3),     // ^C
    (S::VQUIT, 28),    // ^\
    (S::VERASE, 127),  // DEL
    (S::VKILL, 21),    // ^U
    (S::VEOF, 4),      // ^D
    (S::VSTART, 17),   // ^Q
    (S::VSTOP, 19),    // ^S
    (S::VSUSP, 26),    // ^Z
    (S::VREPRINT, 18), // ^R
    (S::VWERASE, 23),  // ^W
    (S::VLNEXT, 22),   // ^V
    (S::VDISCARD, 15), // ^O
];

#[derive(Clone, Copy, Debug)]
pub struct Flag<T> {
    name: &'static str,
    #[expect(clippy::struct_field_names)]
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

#[derive(Debug)]
enum ControlCharMappingError {
    IntOutOfRange(String),
    MultipleChars(String),
}

enum SpecialSetting {
    Rows(u16),
    Cols(u16),
    Line(u8),
}

enum PrintSetting {
    Size,
}

enum ArgOptions<'a> {
    Flags(AllFlags<'a>),
    Mapping((S, u8)),
    Special(SpecialSetting),
    Print(PrintSetting),
}

impl<'a> From<AllFlags<'a>> for ArgOptions<'a> {
    fn from(flag: AllFlags<'a>) -> Self {
        ArgOptions::Flags(flag)
    }
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
                // default to /dev/tty, if that does not exist then default to stdout
                None => {
                    if let Ok(f) = std::fs::OpenOptions::new()
                        .read(true)
                        .custom_flags(O_NONBLOCK)
                        .open("/dev/tty")
                    {
                        Device::File(f)
                    } else {
                        Device::Stdout(stdout())
                    }
                }
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
            translate!("stty-error-options-mutually-exclusive"),
        ));
    }

    if opts.settings.is_some() && (opts.save || opts.all) {
        return Err(USimpleError::new(
            1,
            translate!("stty-error-output-style-no-modes"),
        ));
    }

    let mut set_arg = SetArg::TCSADRAIN;
    let mut valid_args: Vec<ArgOptions> = Vec::new();

    if let Some(args) = &opts.settings {
        let mut args_iter = args.iter();
        while let Some(&arg) = args_iter.next() {
            match arg {
                "ispeed" | "ospeed" => match args_iter.next() {
                    Some(speed) => {
                        if let Some(baud_flag) = string_to_baud(speed) {
                            valid_args.push(ArgOptions::Flags(baud_flag));
                        } else {
                            return Err(USimpleError::new(
                                1,
                                translate!(
                                    "stty-error-invalid-speed",
                                    "arg" => *arg,
                                    "speed" => *speed,
                                ),
                            ));
                        }
                    }
                    None => {
                        return missing_arg(arg);
                    }
                },
                "line" => match args_iter.next() {
                    Some(line) => match parse_u8_or_err(line) {
                        Ok(n) => valid_args.push(ArgOptions::Special(SpecialSetting::Line(n))),
                        Err(e) => return Err(USimpleError::new(1, e)),
                    },
                    None => {
                        return missing_arg(arg);
                    }
                },
                "min" => match args_iter.next() {
                    Some(min) => match parse_u8_or_err(min) {
                        Ok(n) => {
                            valid_args.push(ArgOptions::Mapping((S::VMIN, n)));
                        }
                        Err(e) => return Err(USimpleError::new(1, e)),
                    },
                    None => {
                        return missing_arg(arg);
                    }
                },
                "time" => match args_iter.next() {
                    Some(time) => match parse_u8_or_err(time) {
                        Ok(n) => valid_args.push(ArgOptions::Mapping((S::VTIME, n))),
                        Err(e) => return Err(USimpleError::new(1, e)),
                    },
                    None => {
                        return missing_arg(arg);
                    }
                },
                "rows" => {
                    if let Some(rows) = args_iter.next() {
                        if let Some(n) = parse_rows_cols(rows) {
                            valid_args.push(ArgOptions::Special(SpecialSetting::Rows(n)));
                        } else {
                            return invalid_integer_arg(rows);
                        }
                    } else {
                        return missing_arg(arg);
                    }
                }
                "columns" | "cols" => {
                    if let Some(cols) = args_iter.next() {
                        if let Some(n) = parse_rows_cols(cols) {
                            valid_args.push(ArgOptions::Special(SpecialSetting::Cols(n)));
                        } else {
                            return invalid_integer_arg(cols);
                        }
                    } else {
                        return missing_arg(arg);
                    }
                }
                "drain" => {
                    set_arg = SetArg::TCSADRAIN;
                }
                "-drain" => {
                    set_arg = SetArg::TCSANOW;
                }
                "size" => {
                    valid_args.push(ArgOptions::Print(PrintSetting::Size));
                }
                _ => {
                    // control char
                    if let Some(char_index) = cc_to_index(arg) {
                        if let Some(mapping) = args_iter.next() {
                            let cc_mapping = string_to_control_char(mapping).map_err(|e| {
                                let message = match e {
                                    ControlCharMappingError::IntOutOfRange(val) => {
                                        translate!(
                                            "stty-error-invalid-integer-argument-value-too-large",
                                            "value" => format!("'{val}'")
                                        )
                                    }
                                    ControlCharMappingError::MultipleChars(val) => {
                                        translate!(
                                            "stty-error-invalid-integer-argument",
                                            "value" => format!("'{val}'")
                                        )
                                    }
                                };
                                USimpleError::new(1, message)
                            })?;
                            valid_args.push(ArgOptions::Mapping((char_index, cc_mapping)));
                        } else {
                            return missing_arg(arg);
                        }
                    // baud rate
                    } else if let Some(baud_flag) = string_to_baud(arg) {
                        valid_args.push(ArgOptions::Flags(baud_flag));
                    // non control char flag
                    } else if let Some(flag) = string_to_flag(arg) {
                        let remove_group = match flag {
                            AllFlags::Baud(_) => false,
                            AllFlags::ControlFlags((flag, remove)) => {
                                check_flag_group(flag, remove)
                            }
                            AllFlags::InputFlags((flag, remove)) => check_flag_group(flag, remove),
                            AllFlags::LocalFlags((flag, remove)) => check_flag_group(flag, remove),
                            AllFlags::OutputFlags((flag, remove)) => check_flag_group(flag, remove),
                        };
                        if remove_group {
                            return invalid_arg(arg);
                        }
                        valid_args.push(flag.into());
                    // combination setting
                    } else if let Some(combo) = string_to_combo(arg) {
                        valid_args.append(&mut combo_to_flags(combo));
                    } else {
                        return invalid_arg(arg);
                    }
                }
            }
        }

        // TODO: Figure out the right error message for when tcgetattr fails
        let mut termios = tcgetattr(opts.file.as_fd()).expect("Could not get terminal attributes");

        // iterate over valid_args, match on the arg type, do the matching apply function
        for arg in &valid_args {
            match arg {
                ArgOptions::Mapping(mapping) => apply_char_mapping(&mut termios, mapping),
                ArgOptions::Flags(flag) => apply_setting(&mut termios, flag),
                ArgOptions::Special(setting) => {
                    apply_special_setting(&mut termios, setting, opts.file.as_raw_fd())?;
                }
                ArgOptions::Print(setting) => {
                    print_special_setting(setting, opts.file.as_raw_fd())?;
                }
            }
        }
        tcsetattr(opts.file.as_fd(), set_arg, &termios)
            .expect("Could not write terminal attributes");
    } else {
        // TODO: Figure out the right error message for when tcgetattr fails
        let termios = tcgetattr(opts.file.as_fd()).expect("Could not get terminal attributes");
        print_settings(&termios, opts).expect("TODO: make proper error here from nix error");
    }
    Ok(())
}

fn missing_arg<T>(arg: &str) -> Result<T, Box<dyn UError>> {
    Err::<T, Box<dyn UError>>(USimpleError::new(
        1,
        translate!(
            "stty-error-missing-argument",
            "arg" => *arg
        ),
    ))
}

fn invalid_arg<T>(arg: &str) -> Result<T, Box<dyn UError>> {
    Err::<T, Box<dyn UError>>(USimpleError::new(
        1,
        translate!(
            "stty-error-invalid-argument",
            "arg" => *arg
        ),
    ))
}

fn invalid_integer_arg<T>(arg: &str) -> Result<T, Box<dyn UError>> {
    Err::<T, Box<dyn UError>>(USimpleError::new(
        1,
        translate!(
            "stty-error-invalid-integer-argument",
            "value" => format!("'{arg}'")
        ),
    ))
}

/// GNU uses different error messages if values overflow or underflow a u8,
/// this function returns the appropriate error message in the case of overflow or underflow, or u8 on success
fn parse_u8_or_err(arg: &str) -> Result<u8, String> {
    arg.parse::<u8>().map_err(|e| match e.kind() {
        IntErrorKind::PosOverflow => translate!("stty-error-invalid-integer-argument-value-too-large", "value" => format!("'{arg}'")),
        _ => translate!("stty-error-invalid-integer-argument",
                        "value" => format!("'{arg}'")),
    })
}

/// GNU uses an unsigned 32-bit integer for row/col sizes, but then wraps around 16 bits
/// this function returns Some(n), where n is a u16 row/col size, or None if the string arg cannot be parsed as a u32
fn parse_rows_cols(arg: &str) -> Option<u16> {
    if let Ok(n) = arg.parse::<u32>() {
        return Some((n % (u16::MAX as u32 + 1)) as u16);
    }
    None
}

fn check_flag_group<T>(flag: &Flag<T>, remove: bool) -> bool {
    remove && flag.group.is_some()
}

fn print_special_setting(setting: &PrintSetting, fd: i32) -> nix::Result<()> {
    match setting {
        PrintSetting::Size => {
            let mut size = TermSize::default();
            unsafe { tiocgwinsz(fd, &raw mut size)? };
            println!("{} {}", size.rows, size.columns);
        }
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
    print!("{} ", translate!("stty-output-speed", "speed" => speed));

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
            print!("{} ", translate!("stty-output-speed", "speed" => (*text)));
            break;
        }
    }

    if opts.all {
        let mut size = TermSize::default();
        unsafe { tiocgwinsz(opts.file.as_raw_fd(), &raw mut size)? };
        print!(
            "{} ",
            translate!("stty-output-rows-columns", "rows" => size.rows, "columns" => size.columns)
        );
    }

    #[cfg(any(target_os = "linux", target_os = "redox"))]
    {
        // For some reason the normal nix Termios struct does not expose the line,
        // so we get the underlying libc::termios struct to get that information.
        let libc_termios: nix::libc::termios = termios.clone().into();
        let line = libc_termios.c_line;
        print!("{}", translate!("stty-output-line", "line" => line));
    }

    println!();
    Ok(())
}

fn cc_to_index(option: &str) -> Option<S> {
    for cc in CONTROL_CHARS {
        if option == cc.0 {
            return Some(cc.1);
        }
    }
    None
}

fn string_to_combo(arg: &str) -> Option<&str> {
    let is_negated = arg.starts_with('-');
    let name = arg.trim_start_matches('-');
    COMBINATION_SETTINGS
        .iter()
        .find(|&&(combo_name, is_negatable)| name == combo_name && (!is_negated || is_negatable))
        .map(|_| arg)
}

fn string_to_baud(arg: &str) -> Option<AllFlags<'_>> {
    // BSDs use a u32 for the baud rate, so any decimal number applies.
    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    if let Ok(n) = arg.parse::<u32>() {
        return Some(AllFlags::Baud(n));
    }

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    for (text, baud_rate) in BAUD_RATES {
        if *text == arg {
            return Some(AllFlags::Baud(*baud_rate));
        }
    }
    None
}

/// return `Some(flag)` if the input is a valid flag, `None` if not
fn string_to_flag(option: &str) -> Option<AllFlags<'_>> {
    let remove = option.starts_with('-');
    let name = option.trim_start_matches('-');

    for cflag in CONTROL_FLAGS {
        if name == cflag.name {
            return Some(AllFlags::ControlFlags((cflag, remove)));
        }
    }
    for iflag in INPUT_FLAGS {
        if name == iflag.name {
            return Some(AllFlags::InputFlags((iflag, remove)));
        }
    }
    for lflag in LOCAL_FLAGS {
        if name == lflag.name {
            return Some(AllFlags::LocalFlags((lflag, remove)));
        }
    }
    for oflag in OUTPUT_FLAGS {
        if name == oflag.name {
            return Some(AllFlags::OutputFlags((oflag, remove)));
        }
    }
    None
}

fn control_char_to_string(cc: nix::libc::cc_t) -> nix::Result<String> {
    if cc == 0 {
        return Ok(translate!("stty-output-undef"));
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
        // Print only control chars that differ from sane defaults
        let mut printed = false;
        for (text, cc_index) in CONTROL_CHARS {
            let current_val = termios.control_chars[*cc_index as usize];
            let sane_val = get_sane_control_char(*cc_index);

            if current_val != sane_val {
                print!("{text} = {}; ", control_char_to_string(current_val)?);
                printed = true;
            }
        }

        if printed {
            println!();
        }
        return Ok(());
    }

    for (text, cc_index) in CONTROL_CHARS {
        print!(
            "{text} = {}; ",
            control_char_to_string(termios.control_chars[*cc_index as usize])?
        );
    }
    println!(
        "{}",
        translate!("stty-output-min-time",
        "min" => termios.control_chars[S::VMIN as usize],
        "time" => termios.control_chars[S::VTIME as usize]
        )
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
fn apply_setting(termios: &mut Termios, setting: &AllFlags) {
    match setting {
        AllFlags::Baud(_) => apply_baud_rate_flag(termios, setting),
        AllFlags::ControlFlags((setting, disable)) => {
            setting.flag.apply(termios, !disable);
        }
        AllFlags::InputFlags((setting, disable)) => {
            setting.flag.apply(termios, !disable);
        }
        AllFlags::LocalFlags((setting, disable)) => {
            setting.flag.apply(termios, !disable);
        }
        AllFlags::OutputFlags((setting, disable)) => {
            setting.flag.apply(termios, !disable);
        }
    }
}

fn apply_baud_rate_flag(termios: &mut Termios, input: &AllFlags) {
    // BSDs use a u32 for the baud rate, so any decimal number applies.
    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    if let AllFlags::Baud(n) = input {
        cfsetospeed(termios, *n).expect("Failed to set baud rate");
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
    if let AllFlags::Baud(br) = input {
        cfsetospeed(termios, *br).expect("Failed to set baud rate");
    }
}

fn apply_char_mapping(termios: &mut Termios, mapping: &(S, u8)) {
    termios.control_chars[mapping.0 as usize] = mapping.1;
}

fn apply_special_setting(
    _termios: &mut Termios,
    setting: &SpecialSetting,
    fd: i32,
) -> nix::Result<()> {
    let mut size = TermSize::default();
    unsafe { tiocgwinsz(fd, &raw mut size)? };
    match setting {
        SpecialSetting::Rows(n) => size.rows = *n,
        SpecialSetting::Cols(n) => size.columns = *n,
        SpecialSetting::Line(_n) => {
            // nix only defines Termios's `line_discipline` field on these platforms
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                _termios.line_discipline = *_n;
            }
        }
    }
    unsafe { tiocswinsz(fd, &raw mut size)? };
    Ok(())
}

/// GNU stty defines some valid values for the control character mappings
/// 1. Standard character, can be a a single char (ie 'C') or hat notation (ie '^C')
/// 2. Integer
///    a. hexadecimal, prefixed by '0x'
///    b. octal, prefixed by '0'
///    c. decimal, no prefix
/// 3. Disabling the control character: '^-' or 'undef'
///
/// This function returns the ascii value of valid control chars, or [`ControlCharMappingError`] if invalid
fn string_to_control_char(s: &str) -> Result<u8, ControlCharMappingError> {
    if s == "undef" || s == "^-" || s.is_empty() {
        return Ok(0);
    }

    // try to parse integer (hex, octal, or decimal)
    let ascii_num = if let Some(hex) = s.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).ok()
    } else if let Some(octal) = s.strip_prefix("0") {
        if octal.is_empty() {
            Some(0)
        } else {
            u32::from_str_radix(octal, 8).ok()
        }
    } else {
        s.parse::<u32>().ok()
    };

    if let Some(val) = ascii_num {
        if val > 255 {
            return Err(ControlCharMappingError::IntOutOfRange(s.to_string()));
        }
        return Ok(val as u8);
    }
    // try to parse ^<char> or just <char>
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some('^'), Some(c)) => {
            // special case: ascii value of '^?' is greater than '?'
            if c == '?' {
                return Ok(ASCII_DEL);
            }
            // subtract by '@' to turn the char into the ascii value of '^<char>'
            Ok((c.to_ascii_uppercase() as u8).wrapping_sub(b'@'))
        }
        (Some(c), None) => Ok(c as u8),
        (Some(_), Some(_)) => Err(ControlCharMappingError::MultipleChars(s.to_string())),
        _ => unreachable!("No arguments provided: must have been caught earlier"),
    }
}

// decomposes a combination argument into a vec of corresponding flags
fn combo_to_flags(combo: &str) -> Vec<ArgOptions<'_>> {
    let mut flags = Vec::new();
    let mut ccs = Vec::new();
    match combo {
        "lcase" | "LCASE" => {
            flags = vec!["xcase", "iuclc", "olcuc"];
        }
        "-lcase" | "-LCASE" => {
            flags = vec!["-xcase", "-iuclc", "-olcuc"];
        }
        "cbreak" => {
            flags = vec!["-icanon"];
        }
        "-cbreak" => {
            flags = vec!["icanon"];
        }
        "cooked" | "-raw" => {
            flags = vec![
                "brkint", "ignpar", "istrip", "icrnl", "ixon", "opost", "isig", "icanon",
            ];
            ccs = vec![(S::VEOF, "^D"), (S::VEOL, "")];
        }
        "crt" => {
            flags = vec!["echoe", "echoctl", "echoke"];
        }
        "dec" => {
            flags = vec!["echoe", "echoctl", "echoke", "-ixany"];
            ccs = vec![(S::VINTR, "^C"), (S::VERASE, "^?"), (S::VKILL, "^U")];
        }
        "decctlq" => {
            flags = vec!["ixany"];
        }
        "-decctlq" => {
            flags = vec!["-ixany"];
        }
        "ek" => {
            ccs = vec![(S::VERASE, "^?"), (S::VKILL, "^U")];
        }
        "evenp" | "parity" => {
            flags = vec!["parenb", "-parodd", "cs7"];
        }
        "-evenp" | "-parity" => {
            flags = vec!["-parenb", "cs8"];
        }
        "litout" => {
            flags = vec!["-parenb", "-istrip", "-opost", "cs8"];
        }
        "-litout" => {
            flags = vec!["parenb", "istrip", "opost", "cs7"];
        }
        "nl" => {
            flags = vec!["-icrnl", "-onlcr"];
        }
        "-nl" => {
            flags = vec!["icrnl", "-inlcr", "-igncr", "onlcr", "-ocrnl", "-onlret"];
        }
        "oddp" => {
            flags = vec!["parenb", "parodd", "cs7"];
        }
        "-oddp" => {
            flags = vec!["-parenb", "cs8"];
        }
        "pass8" => {
            flags = vec!["-parenb", "-istrip", "cs8"];
        }
        "-pass8" => {
            flags = vec!["parenb", "istrip", "cs7"];
        }
        "raw" | "-cooked" => {
            flags = vec![
                "-ignbrk", "-brkint", "-ignpar", "-parmrk", "-inpck", "-istrip", "-inlcr",
                "-igncr", "-icrnl", "-ixon", "-ixoff", "-icanon", "-opost", "-isig", "-iuclc",
                "-xcase", "-ixany", "-imaxbel",
            ];
            ccs = vec![(S::VMIN, "1"), (S::VTIME, "0")];
        }
        "sane" => {
            flags = vec![
                "cread", "-ignbrk", "brkint", "-inlcr", "-igncr", "icrnl", "icanon", "iexten",
                "echo", "echoe", "echok", "-echonl", "-noflsh", "-ixoff", "-iutf8", "-iuclc",
                "-xcase", "-ixany", "imaxbel", "-olcuc", "-ocrnl", "opost", "-ofill", "onlcr",
                "-onocr", "-onlret", "nl0", "cr0", "tab0", "bs0", "vt0", "ff0", "isig", "-tostop",
                "-ofdel", "-echoprt", "echoctl", "echoke", "-extproc", "-flusho",
            ];
            ccs = vec![
                (S::VINTR, "^C"),
                (S::VQUIT, "^\\"),
                (S::VERASE, "^?"),
                (S::VKILL, "^U"),
                (S::VEOF, "^D"),
                (S::VEOL, ""),
                (S::VEOL2, ""),
                #[cfg(target_os = "linux")]
                (S::VSWTC, ""),
                (S::VSTART, "^Q"),
                (S::VSTOP, "^S"),
                (S::VSUSP, "^Z"),
                (S::VREPRINT, "^R"),
                (S::VWERASE, "^W"),
                (S::VLNEXT, "^V"),
                (S::VDISCARD, "^O"),
            ];
        }
        _ => unreachable!("invalid combination setting: must have been caught earlier"),
    }
    let mut flags = flags
        .iter()
        .filter_map(|f| string_to_flag(f).map(ArgOptions::Flags))
        .collect::<Vec<ArgOptions>>();
    let mut ccs = ccs
        .iter()
        .map(|cc| ArgOptions::Mapping((cc.0, string_to_control_char(cc.1).unwrap())))
        .collect::<Vec<ArgOptions>>();
    flags.append(&mut ccs);
    flags
}

fn get_sane_control_char(cc_index: S) -> u8 {
    for (sane_index, sane_val) in SANE_CONTROL_CHARS {
        if sane_index == cc_index {
            return sane_val;
        }
    }
    // Default values for control chars not in the sane list
    match cc_index {
        S::VEOL => 0,
        S::VEOL2 => 0,
        S::VMIN => 1,
        S::VTIME => 0,
        #[cfg(target_os = "linux")]
        S::VSWTC => 0,
        _ => 0,
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .override_usage(format_usage(&translate!("stty-usage")))
        .about(translate!("stty-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help(translate!("stty-option-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SAVE)
                .short('g')
                .long(options::SAVE)
                .help(translate!("stty-option-save"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .short('F')
                .long(options::FILE)
                .value_hint(clap::ValueHint::FilePath)
                .value_name("DEVICE")
                .help(translate!("stty-option-file")),
        )
        .arg(
            Arg::new(options::SETTINGS)
                .action(ArgAction::Append)
                .allow_hyphen_values(true)
                .help(translate!("stty-option-settings")),
        )
}

impl TermiosFlag for ControlFlags {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.control_flags.contains(*self)
            && group.is_none_or(|g| !termios.control_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.control_flags.set(*self, val);
    }
}

impl TermiosFlag for InputFlags {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.input_flags.contains(*self)
            && group.is_none_or(|g| !termios.input_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.input_flags.set(*self, val);
    }
}

impl TermiosFlag for OutputFlags {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.output_flags.contains(*self)
            && group.is_none_or(|g| !termios.output_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.output_flags.set(*self, val);
    }
}

impl TermiosFlag for LocalFlags {
    fn is_in(&self, termios: &Termios, group: Option<Self>) -> bool {
        termios.local_flags.contains(*self)
            && group.is_none_or(|g| !termios.local_flags.intersects(g - *self))
    }

    fn apply(&self, termios: &mut Termios, val: bool) {
        termios.local_flags.set(*self, val);
    }
}
