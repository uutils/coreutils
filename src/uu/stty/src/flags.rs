// * This file is part of the uutils coreutils package.
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker:ignore parenb parodd cmspar hupcl cstopb cread clocal crtscts CSIZE
// spell-checker:ignore ignbrk brkint ignpar parmrk inpck istrip inlcr igncr icrnl ixoff ixon iuclc ixany imaxbel iutf
// spell-checker:ignore opost olcuc ocrnl onlcr onocr onlret ofill ofdel nldly crdly tabdly bsdly vtdly ffdly
// spell-checker:ignore isig icanon iexten echoe crterase echok echonl noflsh xcase tostop echoprt prterase echoctl ctlecho echoke crtkill flusho extproc

use crate::Flag;
use nix::sys::termios::{ControlFlags as C, InputFlags as I, LocalFlags as L, OutputFlags as O};

pub const CONTROL_FLAGS: [Flag<C>; 12] = [
    Flag::new("parenb", C::PARENB),
    Flag::new("parodd", C::PARODD),
    Flag::new("cmspar", C::CMSPAR),
    Flag::new("cs5", C::CS5).group(C::CSIZE),
    Flag::new("cs6", C::CS6).group(C::CSIZE),
    Flag::new("cs7", C::CS7).group(C::CSIZE),
    Flag::new("cs8", C::CS8).group(C::CSIZE).sane(),
    Flag::new("hupcl", C::HUPCL).sane(),
    Flag::new("cstopb", C::CSTOPB),
    Flag::new("cread", C::CREAD).sane(),
    Flag::new("clocal", C::CLOCAL),
    Flag::new("crtscts", C::CRTSCTS),
];

pub const INPUT_FLAGS: [Flag<I>; 15] = [
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
    Flag::new("iutf8", I::IUTF8),
];

pub const OUTPUT_FLAGS: [Flag<O>; 24] = [
    Flag::new("opost", O::OPOST).sane(),
    Flag::new("olcuc", O::OLCUC),
    Flag::new("ocrnl", O::OCRNL),
    Flag::new("onlcr", O::ONLCR).sane(),
    Flag::new("onocr", O::ONOCR),
    Flag::new("onlret", O::ONLRET),
    Flag::new("ofill", O::OFILL),
    Flag::new("ofdel", O::OFDEL),
    Flag::new("nl0", O::NL0).group(O::NLDLY).sane(),
    Flag::new("nl1", O::NL1).group(O::NLDLY),
    Flag::new("cr0", O::CR0).group(O::CRDLY).sane(),
    Flag::new("cr1", O::CR1).group(O::CRDLY),
    Flag::new("cr2", O::CR2).group(O::CRDLY),
    Flag::new("cr3", O::CR3).group(O::CRDLY),
    Flag::new("tab0", O::TAB0).group(O::TABDLY).sane(),
    Flag::new("tab1", O::TAB1).group(O::TABDLY),
    Flag::new("tab2", O::TAB2).group(O::TABDLY),
    Flag::new("tab3", O::TAB3).group(O::TABDLY),
    Flag::new("bs0", O::BS0).group(O::BSDLY).sane(),
    Flag::new("bs1", O::BS1).group(O::BSDLY),
    Flag::new("vt0", O::VT0).group(O::VTDLY).sane(),
    Flag::new("vt1", O::VT1).group(O::VTDLY),
    Flag::new("ff0", O::FF0).group(O::FFDLY).sane(),
    Flag::new("ff1", O::FF1).group(O::FFDLY),
];

pub const LOCAL_FLAGS: [Flag<L>; 18] = [
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
