// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore parenb parodd cmspar hupcl cstopb cread clocal crtscts CSIZE
// spell-checker:ignore ignbrk brkint ignpar parmrk inpck istrip inlcr igncr icrnl ixoff ixon iuclc ixany imaxbel iutf
// spell-checker:ignore opost olcuc ocrnl onlcr onocr onlret ofdel nldly crdly tabdly bsdly vtdly ffdly
// spell-checker:ignore isig icanon iexten echoe crterase echok echonl noflsh xcase tostop echoprt prterase echoctl ctlecho echoke crtkill flusho extproc
// spell-checker:ignore lnext rprnt susp swtch vdiscard veof veol verase vintr vkill vlnext vquit vreprint vstart vstop vsusp vswtc vwerase werase
// spell-checker:ignore sigquit sigtstp

use crate::Flag;

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "ios",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
use nix::sys::termios::BaudRate;
use nix::sys::termios::{
    ControlFlags as C, InputFlags as I, LocalFlags as L, OutputFlags as O,
    SpecialCharacterIndices as S,
};

pub const CONTROL_FLAGS: &[Flag<C>] = &[
    Flag::new("parenb", C::PARENB),
    Flag::new("parodd", C::PARODD),
    #[cfg(any(
        target_os = "android",
        all(target_os = "linux", not(target_arch = "mips"))
    ))]
    Flag::new("cmspar", C::CMSPAR),
    Flag::new_grouped("cs5", C::CS5, C::CSIZE),
    Flag::new_grouped("cs6", C::CS6, C::CSIZE),
    Flag::new_grouped("cs7", C::CS7, C::CSIZE),
    Flag::new_grouped("cs8", C::CS8, C::CSIZE).sane(),
    Flag::new("hupcl", C::HUPCL),
    Flag::new("cstopb", C::CSTOPB),
    Flag::new("cread", C::CREAD).sane(),
    Flag::new("clocal", C::CLOCAL),
    Flag::new("crtscts", C::CRTSCTS),
];

pub const INPUT_FLAGS: &[Flag<I>] = &[
    Flag::new("ignbrk", I::IGNBRK),
    Flag::new("brkint", I::BRKINT).sane(),
    Flag::new("ignpar", I::IGNPAR),
    Flag::new("parmrk", I::PARMRK),
    Flag::new("inpck", I::INPCK),
    Flag::new("istrip", I::ISTRIP),
    Flag::new("inlcr", I::INLCR),
    Flag::new("igncr", I::IGNCR),
    Flag::new("icrnl", I::ICRNL).sane(),
    Flag::new("ixoff", I::IXOFF),
    Flag::new("tandem", I::IXOFF),
    Flag::new("ixon", I::IXON),
    // not supported by nix
    // Flag::new("iuclc", I::IUCLC),
    Flag::new("ixany", I::IXANY),
    Flag::new("imaxbel", I::IMAXBEL).sane(),
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    Flag::new("iutf8", I::IUTF8),
];

pub const OUTPUT_FLAGS: &[Flag<O>] = &[
    Flag::new("opost", O::OPOST).sane(),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "linux",
        target_os = "openbsd"
    ))]
    Flag::new("olcuc", O::OLCUC),
    Flag::new("ocrnl", O::OCRNL),
    Flag::new("onlcr", O::ONLCR).sane(),
    Flag::new("onocr", O::ONOCR),
    Flag::new("onlret", O::ONLRET),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new("ofdel", O::OFDEL),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("nl0", O::NL0, O::NLDLY).sane(),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("nl1", O::NL1, O::NLDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("cr0", O::CR0, O::CRDLY).sane(),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("cr1", O::CR1, O::CRDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("cr2", O::CR2, O::CRDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("cr3", O::CR3, O::CRDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("tab0", O::TAB0, O::TABDLY).sane(),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("tab1", O::TAB1, O::TABDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("tab2", O::TAB2, O::TABDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("tab3", O::TAB3, O::TABDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("bs0", O::BS0, O::BSDLY).sane(),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("bs1", O::BS1, O::BSDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("vt0", O::VT0, O::VTDLY).sane(),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("vt1", O::VT1, O::VTDLY),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("ff0", O::FF0, O::FFDLY).sane(),
    #[cfg(any(
        target_os = "android",
        target_os = "haiku",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))]
    Flag::new_grouped("ff1", O::FF1, O::FFDLY),
];

pub const LOCAL_FLAGS: &[Flag<L>] = &[
    Flag::new("isig", L::ISIG).sane(),
    Flag::new("icanon", L::ICANON).sane(),
    Flag::new("iexten", L::IEXTEN).sane(),
    Flag::new("echo", L::ECHO).sane(),
    Flag::new("echoe", L::ECHOE).sane(),
    Flag::new("crterase", L::ECHOE).hidden().sane(),
    Flag::new("echok", L::ECHOK).sane(),
    Flag::new("echonl", L::ECHONL),
    Flag::new("noflsh", L::NOFLSH),
    // Not supported by nix
    // Flag::new("xcase", L::XCASE),
    Flag::new("tostop", L::TOSTOP),
    Flag::new("echoprt", L::ECHOPRT),
    Flag::new("prterase", L::ECHOPRT).hidden(),
    Flag::new("echoctl", L::ECHOCTL).sane(),
    Flag::new("ctlecho", L::ECHOCTL).sane().hidden(),
    Flag::new("echoke", L::ECHOKE).sane(),
    Flag::new("crtkill", L::ECHOKE).sane().hidden(),
    Flag::new("flusho", L::FLUSHO),
    Flag::new("extproc", L::EXTPROC),
];

// BSD's use u32 as baud rate, to using the enum is unnecessary.
#[cfg(not(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "ios",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
pub const BAUD_RATES: &[(&str, BaudRate)] = &[
    ("0", BaudRate::B0),
    ("50", BaudRate::B50),
    ("75", BaudRate::B75),
    ("110", BaudRate::B110),
    ("134", BaudRate::B134),
    ("150", BaudRate::B150),
    ("200", BaudRate::B200),
    ("300", BaudRate::B300),
    ("600", BaudRate::B600),
    ("1200", BaudRate::B1200),
    ("1800", BaudRate::B1800),
    ("2400", BaudRate::B2400),
    ("9600", BaudRate::B9600),
    ("19200", BaudRate::B19200),
    ("38400", BaudRate::B38400),
    ("57600", BaudRate::B57600),
    ("115200", BaudRate::B115200),
    ("230400", BaudRate::B230400),
    #[cfg(any(target_os = "android", target_os = "linux"))]
    ("500000", BaudRate::B500000),
    #[cfg(any(target_os = "android", target_os = "linux"))]
    ("576000", BaudRate::B576000),
    #[cfg(any(target_os = "android", target_os = "linux",))]
    ("921600", BaudRate::B921600),
    #[cfg(any(target_os = "android", target_os = "linux"))]
    ("1000000", BaudRate::B1000000),
    #[cfg(any(target_os = "android", target_os = "linux"))]
    ("1152000", BaudRate::B1152000),
    #[cfg(any(target_os = "android", target_os = "linux"))]
    ("1500000", BaudRate::B1500000),
    #[cfg(any(target_os = "android", target_os = "linux"))]
    ("2000000", BaudRate::B2000000),
    #[cfg(any(
        target_os = "android",
        all(target_os = "linux", not(target_arch = "sparc64"))
    ))]
    ("2500000", BaudRate::B2500000),
    #[cfg(any(
        target_os = "android",
        all(target_os = "linux", not(target_arch = "sparc64"))
    ))]
    ("3000000", BaudRate::B3000000),
    #[cfg(any(
        target_os = "android",
        all(target_os = "linux", not(target_arch = "sparc64"))
    ))]
    ("3500000", BaudRate::B3500000),
    #[cfg(any(
        target_os = "android",
        all(target_os = "linux", not(target_arch = "sparc64"))
    ))]
    ("4000000", BaudRate::B4000000),
];
/// Control characters for the stty command.
///
/// This constant provides a mapping between the names of control characters
/// and their corresponding values in the `S` enum.
pub const CONTROL_CHARS: &[(&str, S)] = &[
    // Sends an interrupt signal (SIGINT).
    ("intr", S::VINTR),
    // Sends a quit signal (SIGQUIT).
    ("quit", S::VQUIT),
    // Deletes the last typed character.
    ("erase", S::VERASE),
    // Deletes the current line.
    ("kill", S::VKILL),
    // Signals the end of input.
    ("eof", S::VEOF),
    // Signals the end of line.
    ("eol", S::VEOL),
    // Alternate end-of-line character.
    ("eol2", S::VEOL2),
    // Switch character (only on Linux).
    #[cfg(target_os = "linux")]
    ("swtch", S::VSWTC),
    // Starts output after it has been stopped.
    ("start", S::VSTART),
    // Stops output.
    ("stop", S::VSTOP),
    // Sends a suspend signal (SIGTSTP).
    ("susp", S::VSUSP),
    // Reprints the current line.
    ("rprnt", S::VREPRINT),
    // Deletes the last word typed.
    ("werase", S::VWERASE),
    // Enters literal mode (next character is taken literally).
    ("lnext", S::VLNEXT),
    // Discards the current line.
    ("discard", S::VDISCARD),
];
