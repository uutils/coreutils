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
// spell-checker:ignore cbreak decctlq evenp litout oddp
// spell-checker:ignore VDSUSP VMIN VSTATUS VTIME VSWTCH

use crate::Flag;

use rustix::termios::{
    ControlModes as C, InputModes as I, LocalModes as L, OutputModes as O, SpecialCodeIndex as S,
};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum BaudType {
    Input,
    Output,
    Both,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum AllFlags<'a> {
    Baud(u32, BaudType),
    ControlFlags((&'a Flag<C>, bool)),
    InputFlags((&'a Flag<I>, bool)),
    LocalFlags((&'a Flag<L>, bool)),
    OutputFlags((&'a Flag<O>, bool)),
}

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
    #[cfg(not(target_os = "redox"))]
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
    #[cfg(not(target_os = "redox"))]
    Flag::new("ixany", I::IXANY),
    #[cfg(not(target_os = "redox"))]
    Flag::new("imaxbel", I::IMAXBEL).sane(),
    #[cfg(any(target_vendor = "apple", target_os = "android", target_os = "linux"))]
    Flag::new("iutf8", I::IUTF8),
];

// workaround for unexposed const of OutputModes <https://github.com/bytecodealliance/rustix/issues/1706>
#[allow(unused_macros)]
macro_rules! o {
    ($val:expr) => {
        O::from_bits_retain($val as _)
    };
}

#[allow(clippy::unnecessary_cast)]
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
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new("ofdel", o!(libc::OFDEL)),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("nl0", o!(libc::NL0), o!(libc::NLDLY)).sane(),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("nl1", o!(libc::NL1), o!(libc::NLDLY)),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("cr0", o!(libc::CR0), o!(libc::CRDLY)).sane(),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("cr1", o!(libc::CR1), o!(libc::CRDLY)),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("cr2", o!(libc::CR2), o!(libc::CRDLY)),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("cr3", o!(libc::CR3), o!(libc::CRDLY)),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("tab0", O::TAB0, O::TABDLY).sane(),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("tab1", o!(libc::TAB1), O::TABDLY),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("tab2", o!(libc::TAB2), O::TABDLY),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("tab3", o!(libc::TAB3), o!(libc::BSDLY)),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("bs0", o!(libc::BS0), o!(libc::BSDLY)).sane(),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("bs1", o!(libc::BS1), o!(libc::BSDLY)),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("vt0", o!(libc::VT0), o!(libc::VTDLY)).sane(),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("vt1", o!(libc::VT1), o!(libc::VTDLY)),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("ff0", o!(libc::FF0), o!(libc::FFDLY)).sane(),
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "haiku",
        target_os = "linux"
    ))]
    Flag::new_grouped("ff1", o!(libc::FF1), o!(libc::FFDLY)),
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
    #[cfg(not(any(target_os = "cygwin", target_os = "redox")))]
    Flag::new("echoprt", L::ECHOPRT),
    #[cfg(not(any(target_os = "cygwin", target_os = "redox")))]
    Flag::new("prterase", L::ECHOPRT).hidden(),
    #[cfg(not(target_os = "redox"))]
    Flag::new("echoctl", L::ECHOCTL).sane(),
    #[cfg(not(target_os = "redox"))]
    Flag::new("ctlecho", L::ECHOCTL).sane().hidden(),
    #[cfg(not(target_os = "redox"))]
    Flag::new("echoke", L::ECHOKE).sane(),
    #[cfg(not(target_os = "redox"))]
    Flag::new("crtkill", L::ECHOKE).sane().hidden(),
    #[cfg(not(target_os = "redox"))]
    Flag::new("flusho", L::FLUSHO),
    #[cfg(not(any(target_os = "cygwin", target_os = "redox")))]
    Flag::new("extproc", L::EXTPROC),
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

/// This constant lists all possible combination settings, using a bool to represent if the setting is negatable
pub const COMBINATION_SETTINGS: &[(&str, bool)] = &[
    ("LCASE", true),
    ("lcase", true),
    ("cbreak", true),
    ("cooked", true),
    ("crt", false),
    ("dec", false),
    ("decctlq", true),
    ("ek", false),
    ("evenp", true),
    ("litout", true),
    ("nl", true),
    ("oddp", true),
    ("parity", true),
    ("pass8", true),
    ("raw", true),
    ("sane", false),
    ("tabs", true),
];

// rustix::termios::SpecialCodeIndex, but ordered as same as C header
#[cfg(bsd)] // different order with Linux
pub const SPECIAL_INDICES: &[S] = &[
    S::VEOF,
    S::VEOL,
    S::VEOL2,
    S::VERASE,
    S::VWERASE,
    S::VKILL,
    S::VREPRINT,
    S::VINTR,
    S::VQUIT,
    S::VSUSP,
    S::VDSUSP,
    S::VSTART,
    S::VSTOP,
    S::VLNEXT,
    S::VDISCARD,
    S::VMIN,
    S::VTIME,
    S::VSTATUS,
];

#[cfg(not(bsd))]
pub const SPECIAL_INDICES: &[S] = &[
    S::VINTR,
    S::VQUIT,
    S::VERASE,
    S::VKILL,
    S::VEOF,
    S::VTIME,
    S::VMIN,
    #[cfg(not(any(target_os = "haiku", target_os = "hurd")))]
    S::VSWTC,
    S::VSTART,
    S::VSTOP,
    S::VSUSP,
    S::VEOL,
    #[cfg(not(target_os = "haiku"))]
    S::VREPRINT,
    #[cfg(not(target_os = "haiku"))]
    S::VDISCARD,
    #[cfg(not(target_os = "haiku"))]
    S::VWERASE,
    #[cfg(not(target_os = "haiku"))]
    S::VLNEXT,
    S::VEOL2,
    #[cfg(target_os = "haiku")]
    S::VSWTCH,
    #[cfg(target_os = "hurd")]
    S::VDSUSP,
    #[cfg(any(target_os = "hurd", target_os = "illumos"))]
    S::VSTATUS,
];
