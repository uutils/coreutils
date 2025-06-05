// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore clocal erange tcgetattr tcsetattr tcsanow tiocgwinsz tiocswinsz cfgetospeed cfsetospeed ushort vmin vtime cflag lflag

mod flags;

use crate::flags::AllFlags;
use clap::{Arg, ArgAction, ArgMatches, Command};
use nix::libc::{O_NONBLOCK, TIOCGWINSZ, TIOCSWINSZ, c_ushort};
use nix::sys::termios::{
    ControlFlags, InputFlags, LocalFlags, OutputFlags, SpecialCharacterIndices, Termios,
    cfgetospeed, cfsetospeed, tcgetattr, tcsetattr,
};
use nix::{ioctl_read_bad, ioctl_write_ptr_bad};
use std::fs::File;
use std::io::{self, Stdout, stdout};
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;
use uucore::locale::get_message;

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

enum ControlCharMappingError {
    IntOutOfRange,
    MultipleChars,
}

enum ArgOptions<'a> {
    Flags(AllFlags<'a>),
    Mapping((SpecialCharacterIndices, u8)),
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
            "the options for verbose and stty-readable output styles are mutually exclusive",
        ));
    }

    if opts.settings.is_some() && (opts.save || opts.all) {
        return Err(USimpleError::new(
            1,
            "when specifying an output style, modes may not be set",
        ));
    }

    let mut valid_args: Vec<ArgOptions> = Vec::new();

    if let Some(args) = &opts.settings {
        let mut args_iter = args.iter();
        // iterate over args: skip to next arg if current one is a control char
        while let Some(arg) = args_iter.next() {
            // control char
            if let Some(char_index) = cc_to_index(arg) {
                if let Some(mapping) = args_iter.next() {
                    let cc_mapping = string_to_control_char(mapping).map_err(|e| {
                        let message = match e {
                            ControlCharMappingError::IntOutOfRange => format!(
                                "invalid integer argument: '{mapping}': Value too large for defined data type"
                            ),
                            ControlCharMappingError::MultipleChars => {
                                format!("invalid integer argument: '{mapping}'")
                            }
                        };
                        USimpleError::new(1, message)
                    })?;
                    valid_args.push(ArgOptions::Mapping((char_index, cc_mapping)));
                } else {
                    return Err(USimpleError::new(1, format!("missing argument to '{arg}'")));
                }
            // non control char flag
            } else if let Some(flag) = string_to_flag(arg) {
                let remove_group = match flag {
                    AllFlags::Baud(_) => false,
                    AllFlags::ControlFlags((flag, remove)) => check_flag_group(flag, remove),
                    AllFlags::InputFlags((flag, remove)) => check_flag_group(flag, remove),
                    AllFlags::LocalFlags((flag, remove)) => check_flag_group(flag, remove),
                    AllFlags::OutputFlags((flag, remove)) => check_flag_group(flag, remove),
                };
                if remove_group {
                    return Err(USimpleError::new(1, format!("invalid argument '{arg}'")));
                }
                valid_args.push(flag.into());
            // not a valid control char or flag
            } else {
                return Err(USimpleError::new(1, format!("invalid argument '{arg}'")));
            }
        }

        // TODO: Figure out the right error message for when tcgetattr fails
        let mut termios = tcgetattr(opts.file.as_fd()).expect("Could not get terminal attributes");

        // iterate over valid_args, match on the arg type, do the matching apply function
        for arg in &valid_args {
            match arg {
                ArgOptions::Mapping(mapping) => apply_char_mapping(&mut termios, mapping),
                ArgOptions::Flags(flag) => apply_setting(&mut termios, flag),
            }
        }
        tcsetattr(
            opts.file.as_fd(),
            nix::sys::termios::SetArg::TCSANOW,
            &termios,
        )
        .expect("Could not write terminal attributes");
    } else {
        // TODO: Figure out the right error message for when tcgetattr fails
        let termios = tcgetattr(opts.file.as_fd()).expect("Could not get terminal attributes");
        print_settings(&termios, opts).expect("TODO: make proper error here from nix error");
    }
    Ok(())
}

fn check_flag_group<T>(flag: &Flag<T>, remove: bool) -> bool {
    remove && flag.group.is_some()
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
        unsafe { tiocgwinsz(opts.file.as_raw_fd(), &raw mut size)? };
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

fn cc_to_index(option: &str) -> Option<SpecialCharacterIndices> {
    for cc in CONTROL_CHARS {
        if option == cc.0 {
            return Some(cc.1);
        }
    }
    None
}

// return Some(flag) if the input is a valid flag, None if not
fn string_to_flag(option: &str) -> Option<AllFlags> {
    // BSDs use a u32 for the baud rate, so any decimal number applies.
    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    if let Ok(n) = option.parse::<u32>() {
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
        if *text == option {
            return Some(AllFlags::Baud(*baud_rate));
        }
    }

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

fn apply_char_mapping(termios: &mut Termios, mapping: &(SpecialCharacterIndices, u8)) {
    termios.control_chars[mapping.0 as usize] = mapping.1;
}

// GNU stty defines some valid values for the control character mappings
// 1. Standard character, can be a a single char (ie 'C') or hat notation (ie '^C')
// 2. Integer
//      a. hexadecimal, prefixed by '0x'
//      b. octal, prefixed by '0'
//      c. decimal, no prefix
// 3. Disabling the control character: '^-' or 'undef'
//
// This function returns the ascii value of valid control chars, or ControlCharMappingError if invalid
fn string_to_control_char(s: &str) -> Result<u8, ControlCharMappingError> {
    if s == "undef" || s == "^-" {
        return Ok(0);
    }

    // try to parse integer (hex, octal, or decimal)
    let ascii_num = if let Some(hex) = s.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).ok()
    } else if let Some(octal) = s.strip_prefix("0") {
        u32::from_str_radix(octal, 8).ok()
    } else {
        s.parse::<u32>().ok()
    };

    if let Some(val) = ascii_num {
        if val > 255 {
            return Err(ControlCharMappingError::IntOutOfRange);
        } else {
            return Ok(val as u8);
        }
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
        (Some(_), Some(_)) => Err(ControlCharMappingError::MultipleChars),
        _ => unreachable!("No arguments provided: must have been caught earlier"),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .override_usage(format_usage(&get_message("stty-usage")))
        .about(get_message("stty-about"))
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
