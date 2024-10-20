// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore clocal erange tcgetattr tcsetattr tcsanow tiocgwinsz tiocswinsz cfgetospeed cfsetospeed ushort vmin vtime ixon pathconf endregion

mod flags;
mod generated;

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use nix::libc::{c_ushort, O_NONBLOCK, TIOCGWINSZ, TIOCSWINSZ};
use nix::sys::termios::{
    cfgetospeed, cfsetospeed, tcgetattr, tcsetattr, ControlFlags, InputFlags, LocalFlags,
    OutputFlags, SetArg, SpecialCharacterIndices, Termios,
};
use nix::unistd::{pathconf, PathconfVar};
use nix::{ioctl_read_bad, ioctl_write_ptr_bad};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::io::{self, Stdin, StdoutLock};
use std::ops::ControlFlow;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::OnceLock;
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
    Stdin(Stdin),
}

impl AsFd for Device {
    fn as_fd(&self) -> BorrowedFd<'_> {
        match self {
            Self::File(f) => f.as_fd(),
            Self::Stdin(stdin) => stdin.as_fd(),
        }
    }
}

impl AsRawFd for Device {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Self::File(f) => f.as_raw_fd(),
            Self::Stdin(stdin) => stdin.as_raw_fd(),
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
                None => Device::Stdin(io::stdin()),
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
    // Manually fix this edge case:
    //
    // stty -- -ixon
    let end_of_options_os_str = OsStr::new("--");

    // Ignore the end of options delimiter ("--") and everything after, as GNU Core Utilities does
    let fixed_args = args.take_while(|os| os.as_os_str() != end_of_options_os_str);

    let matches = uu_app().try_get_matches_from(fixed_args)?;

    let options = Options::from(&matches)?;

    stty(options)
}

fn stty(options: Options) -> UResult<()> {
    if options.save && options.all {
        return Err(USimpleError::new(
            1,
            "the options for verbose and stty-readable output styles are mutually exclusive",
        ));
    }

    if options.settings.is_some() && (options.save || options.all) {
        return Err(USimpleError::new(
            1,
            "when specifying an output style, modes may not be set",
        ));
    }

    // TODO: Figure out the right error message for when tcgetattr fails
    let mut termios = match tcgetattr(options.file.as_fd()) {
        Ok(te) => te,
        Err(er) => {
            return Err(USimpleError::new(
                1,
                format!("could not get terminal attributes: errno {er}"),
            ));
        }
    };

    if let Some(settings) = options.settings {
        let settings_len = settings.len();

        let mut special =
            Vec::<(SpecialCharacterIndices, SpecialCharacterAssignment)>::with_capacity(
                settings_len,
            );
        let mut rest = Vec::<&str>::with_capacity(settings_len);

        let special_character_indices_hash_map = get_special_character_indices_hash_map();

        let mut peekable = settings.into_iter().peekable();

        while let Some(&st) = peekable.peek() {
            peekable.next();

            if let Some(&sp) = special_character_indices_hash_map.get(st) {
                let Some(binding) = peekable.next() else {
                    return Err(USimpleError::new(
                        1_i32,
                        format!("missing argument to '{st}'"),
                    ));
                };

                let spe = match parse_special_character_assignment(binding) {
                    Ok(spec) => spec,
                    Err(st) => {
                        return Err(USimpleError::new(1, st));
                    }
                };

                special.push((sp, spe));
            } else {
                rest.push(st);
            }
        }

        for setting in rest {
            match apply_setting(&mut termios, setting) {
                ControlFlow::Break(re) => {
                    if re? {
                        // The setting was successfully applied
                        continue;
                    } else {
                        // All attempts to apply the setting failed
                        return Err(USimpleError::new(
                            1,
                            format!("invalid argument '{setting}'"),
                        ));
                    }
                }
                ControlFlow::Continue(()) => {
                    // Should be unreachable
                    debug_assert!(false);
                }
            }
        }

        for (sp, spe) in special {
            let sp_usize = sp as usize;

            let Some(control_char) = termios.control_chars.get_mut(sp_usize) else {
                return Err(USimpleError::new(
                    1,
                    format!("failed to get special character to assign: '{sp_usize}'"),
                ));
            };

            match spe {
                SpecialCharacterAssignment::DisablingAssignment => {
                    match get_disabling_assignment() {
                        Ok(ue) => {
                            *control_char = ue;
                        }
                        Err(bo) => {
                            return Err(USimpleError::new(
                                1,
                                format!("failed to disable a special character: {bo}"),
                            ));
                        }
                    }
                }
                SpecialCharacterAssignment::AssignTo(ue) => {
                    *control_char = ue;
                }
            }
        }

        if let Err(er) = tcsetattr(options.file.as_fd(), SetArg::TCSANOW, &termios) {
            return Err(USimpleError::new(
                1,
                format!("Could not write terminal attributes: errno {er}"),
            ));
        }
    } else {
        #[allow(clippy::collapsible_else_if)]
        if let Err(bo) = print_settings(&termios, &options) {
            return Err(USimpleError::new(
                1,
                format!("failed to print settings: {bo}"),
            ));
        }
    }

    Ok(())
}

fn print_terminal_size(
    stdout_lock: &mut StdoutLock,
    termios: &Termios,
    options: &Options,
) -> UResult<()> {
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
    write!(stdout_lock, "speed {speed} baud; ")?;

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
            write!(stdout_lock, "speed {text} baud; ")?;

            break;
        }
    }

    if options.all {
        let mut size = TermSize::default();

        unsafe { tiocgwinsz(options.file.as_raw_fd(), &mut size as *mut _)? };

        write!(
            stdout_lock,
            "rows {}; columns {}; ",
            size.rows, size.columns
        )?;
    }

    #[cfg(any(target_os = "linux", target_os = "redox"))]
    {
        // For some reason the normal nix Termios struct does not expose the line,
        // so we get the underlying libc::termios struct to get that information.
        let libc_termios: nix::libc::termios = termios.clone().into();
        let line = libc_termios.c_line;

        write!(stdout_lock, "line = {line};")?;
    }

    writeln!(stdout_lock)?;

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

fn print_control_chars(
    stdout_lock: &mut StdoutLock,
    termios: &Termios,
    options: &Options,
) -> UResult<()> {
    if !options.all {
        // TODO: this branch should print values that differ from defaults
        return Ok(());
    }

    for (text, cc_index) in CONTROL_CHARS {
        write!(
            stdout_lock,
            "{text} = {}; ",
            control_char_to_string(termios.control_chars[*cc_index as usize])?
        )?;
    }

    writeln!(
        stdout_lock,
        "min = {}; time = {};",
        termios.control_chars[SpecialCharacterIndices::VMIN as usize],
        termios.control_chars[SpecialCharacterIndices::VTIME as usize]
    )?;

    Ok(())
}

fn print_in_save_format(stdout_lock: &mut StdoutLock, termios: &Termios) -> UResult<()> {
    write!(
        stdout_lock,
        "{:x}:{:x}:{:x}:{:x}",
        termios.input_flags.bits(),
        termios.output_flags.bits(),
        termios.control_flags.bits(),
        termios.local_flags.bits()
    )?;

    for cc in termios.control_chars {
        write!(stdout_lock, ":{cc:x}")?;
    }

    writeln!(stdout_lock)?;

    Ok(())
}

fn print_settings(termios: &Termios, options: &Options) -> UResult<()> {
    let mut stdout_lock = io::stdout().lock();

    if options.save {
        print_in_save_format(&mut stdout_lock, termios)?;
    } else {
        print_terminal_size(&mut stdout_lock, termios, options)?;
        print_control_chars(&mut stdout_lock, termios, options)?;
        print_flags(&mut stdout_lock, termios, options, CONTROL_FLAGS)?;
        print_flags(&mut stdout_lock, termios, options, INPUT_FLAGS)?;
        print_flags(&mut stdout_lock, termios, options, OUTPUT_FLAGS)?;
        print_flags(&mut stdout_lock, termios, options, LOCAL_FLAGS)?;
    }

    Ok(())
}

fn print_flags<T: TermiosFlag>(
    stdout_lock: &mut StdoutLock,
    termios: &Termios,
    options: &Options,
    flags: &[Flag<T>],
) -> UResult<()> {
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
            if val && (!sane || options.all) {
                write!(stdout_lock, "{name} ")?;

                printed = true;
            }
        } else if options.all || val != sane {
            if !val {
                write!(stdout_lock, "-")?;
            }

            write!(stdout_lock, "{name} ")?;

            printed = true;
        }
    }

    if printed {
        writeln!(stdout_lock)?;
    }

    Ok(())
}

/// Apply a single setting
///
/// The value inside the `Break` variant of the `ControlFlow` indicates whether
/// the setting has been applied.
fn apply_setting(termios: &mut Termios, setting: &str) -> ControlFlow<UResult<bool>> {
    apply_baud_rate_flag(termios, setting)?;

    let (remove, name) = match setting.strip_prefix('-') {
        Some(st) => (true, st),
        None => (false, setting),
    };

    apply_flag(termios, CONTROL_FLAGS, name, remove)?;
    apply_flag(termios, INPUT_FLAGS, name, remove)?;
    apply_flag(termios, OUTPUT_FLAGS, name, remove)?;
    apply_flag(termios, LOCAL_FLAGS, name, remove)?;

    ControlFlow::Break(Ok(false))
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
) -> ControlFlow<UResult<bool>> {
    for Flag {
        name, flag, group, ..
    } in flags
    {
        if input == *name {
            // Flags with groups cannot be removed
            // Since the name matches, we can short circuit and don't have to check the other flags.
            if remove && group.is_some() {
                return ControlFlow::Break(Ok(false));
            }

            // If there is a group, the bits for that group should be cleared before applying the flag
            if let Some(group) = group {
                group.apply(termios, false);
            }

            flag.apply(termios, !remove);

            return ControlFlow::Break(Ok(true));
        }
    }

    ControlFlow::Continue(())
}

fn apply_baud_rate_flag(termios: &mut Termios, input: &str) -> ControlFlow<UResult<bool>> {
    fn map_cfsetospeed_result(result: nix::Result<()>) -> UResult<bool> {
        match result {
            Ok(()) => Ok(true),
            Err(er) => Err(USimpleError::new(
                1,
                format!("failed to set baud rate: errno {er}"),
            )),
        }
    }

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
        let result = map_cfsetospeed_result(cfsetospeed(termios, n));

        return ControlFlow::Break(result);
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
            let result = map_cfsetospeed_result(cfsetospeed(termios, *baud_rate));

            return ControlFlow::Break(result);
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
                // Allows e.g. "stty -ixon" to work
                .allow_hyphen_values(true)
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

enum SpecialCharacterAssignment {
    DisablingAssignment,
    AssignTo(u8),
}

// See explanation above "Table: Circumflex Control Characters in stty" in
// https://pubs.opengroup.org/onlinepubs/9799919799/utilities/stty.html
//
// Also:
//
// "They are set with the syntax ‘name value’, where the names are listed below and the value can be given either literally, in hat notation (‘^c’), or as an integer which may start with ‘0x’ to indicate hexadecimal, ‘0’ to indicate octal, or any other digit to indicate decimal."
// https://www.gnu.org/software/coreutils/manual/html_node/Characters.html
//
// GNU Core Utilities allows all of these for disabling assignment:
// "", "^-", "undef"
fn parse_special_character_assignment(
    assignment_str: &str,
) -> Result<SpecialCharacterAssignment, String> {
    use crate::generated::CIRCUMFLEX_CONTROL_CHARACTERS_TABLE_MAP;

    let make_error = || Err(format!("invalid integer argument: '{assignment_str}'"));

    fn ensure_one_byte(ch: char) -> Result<u8, String> {
        let mut encoding_buffer = [0_u8; 4_usize];

        let st = ch.encode_utf8(&mut encoding_buffer);

        match st.as_bytes() {
            &[ue] => {
                Ok(ue)
            }
            _ => {
                Err(format!("Character '{ch:?}' cannot be used for special control character assignment, because it has a multi-byte UTF-8 representation"))
            }
        }
    }

    let mut chars = assignment_str.chars();

    // e.g. GNU Core utilities treats:
    //
    // stty intr ''
    //
    // as a disabling assignment
    let Some(first_char) = chars.next() else {
        return Ok(SpecialCharacterAssignment::DisablingAssignment);
    };

    let first_char_byte = ensure_one_byte(first_char)?;

    let after_first_char_chars = chars.clone();

    let Some(second_char) = chars.next() else {
        // Single character is treated literally
        return Ok(SpecialCharacterAssignment::AssignTo(first_char_byte));
    };

    let second_char_byte = ensure_one_byte(second_char)?;

    match (first_char_byte, second_char_byte) {
        (b'0', b'x') => {
            // Hexadecimal
            let result = u8::from_str_radix(chars.as_str(), 16_u32);

            return if let Ok(ue) = result {
                Ok(SpecialCharacterAssignment::AssignTo(ue))
            } else {
                make_error()
            };
        }
        (b'0', _) => {
            // Octal
            let result = u8::from_str_radix(after_first_char_chars.as_str(), 8_u32);

            return if let Ok(ue) = result {
                Ok(SpecialCharacterAssignment::AssignTo(ue))
            } else {
                make_error()
            };
        }
        _ => {
            // Continue
        }
    };

    if let Some(third_char) = chars.next() {
        let third_char_byte = ensure_one_byte(third_char)?;

        if let (b'u', b'n', b'd') = (first_char_byte, second_char_byte, third_char_byte) {
            // Look for "undef"
            if chars.as_str() == "ef" {
                return Ok(SpecialCharacterAssignment::DisablingAssignment);
            }
        }
    } else {
        match (first_char_byte, second_char_byte) {
            (b'^', b'-') => return Ok(SpecialCharacterAssignment::DisablingAssignment),
            (b'^', ue) => {
                // Circumflex
                let map: &'static phf::Map<u8, u8> = &CIRCUMFLEX_CONTROL_CHARACTERS_TABLE_MAP;

                if let Some(uei) = map.get(&ue) {
                    return Ok(SpecialCharacterAssignment::AssignTo(*uei));
                }
            }
            _ => {}
        }
    }

    make_error()
}

fn get_special_character_indices_hash_map() -> HashMap<&'static str, SpecialCharacterIndices> {
    let mut hash_map = HashMap::<&'static str, SpecialCharacterIndices>::with_capacity(18_usize);

    let mut insert = |key: &'static str, value: SpecialCharacterIndices| {
        debug_assert!(hash_map.insert(key, value).is_none());
    };

    /* #region POSIX */
    {
        // "Table: Control Character Names in stty"
        // https://pubs.opengroup.org/onlinepubs/9799919799/utilities/stty.html

        // spell-checker:disable
        insert("eof", SpecialCharacterIndices::VEOF);
        insert("eol", SpecialCharacterIndices::VEOL);
        insert("erase", SpecialCharacterIndices::VERASE);
        insert("intr", SpecialCharacterIndices::VINTR);
        insert("kill", SpecialCharacterIndices::VKILL);
        insert("quit", SpecialCharacterIndices::VQUIT);
        insert("susp", SpecialCharacterIndices::VSUSP);
        insert("start", SpecialCharacterIndices::VSTART);
        insert("stop", SpecialCharacterIndices::VSTOP);
        // spell-checker:enable
    }
    /* #endregion */

    /* #region Non-POSIX */
    {
        // https://www.gnu.org/software/coreutils/manual/html_node/Characters.html

        // spell-checker:disable
        insert("eol2", SpecialCharacterIndices::VEOL2);

        #[cfg(not(any(target_os = "aix", target_os = "haiku")))]
        {
            insert("discard", SpecialCharacterIndices::VDISCARD);
        }

        #[cfg(target_os = "linux")]
        {
            insert("swtch", SpecialCharacterIndices::VSWTC);
        }

        // TODO
        // Haiku and Solaris
        // insert("swtch", SpecialCharacterIndices::VSWTCH);

        // TODO
        // BSD and Solaris
        // insert("status", SpecialCharacterIndices::VSTATUS);

        // TODO
        // AIX, BSD, and Solaris
        // insert("dsusp", SpecialCharacterIndices::VDSUSP);

        #[cfg(not(target_os = "haiku"))]
        {
            insert("rprnt", SpecialCharacterIndices::VREPRINT);
        }

        #[cfg(not(any(target_os = "aix", target_os = "haiku")))]
        {
            insert("werase", SpecialCharacterIndices::VWERASE);
        }

        #[cfg(not(target_os = "haiku"))]
        {
            insert("lnext", SpecialCharacterIndices::VLNEXT);
        }
        // spell-checker:enable
    }
    /* #endregion */

    hash_map
}

fn get_disabling_assignment() -> Result<u8, Box<dyn Error>> {
    static ONCE_LOCK: OnceLock<Option<u8>> = OnceLock::<Option<u8>>::new();

    let option = match ONCE_LOCK.get() {
        Some(op) => op,
        None => {
            let path_buf = env::current_dir()?;

            // spell-checker:disable-next-line
            let op = pathconf(path_buf.as_path(), PathconfVar::_POSIX_VDISABLE)?;

            let opt = match op {
                Some(is) => Some(u8::try_from(is)?),
                None => None,
            };

            ONCE_LOCK.get_or_init(|| opt)
        }
    };

    match option {
        Some(ue) => Ok(*ue),
        None => Err(Box::from(
            "Disabling special characters is not supported on this system",
        )),
    }
}
