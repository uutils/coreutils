// * This file is part of the uutils coreutils package.
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker:ignore parenb parodd cmspar hupcl cstopb cread clocal crtscts
// spell-checker:ignore ignbrk brkint ignpar parmrk inpck istrip inlcr igncr icrnl ixoff ixon iuclc ixany imaxbel iutf
// spell-checker:ignore opost olcuc ocrnl onlcr onocr onlret ofill ofdel
// spell-checker:ignore isig icanon iexten echoe crterase echok echonl noflsh xcase tostop echoprt prterase echoctl ctlecho echoke crtkill flusho extproc

use crate::Flag;
use nix::sys::termios::{ControlFlags, InputFlags, LocalFlags, OutputFlags};

pub const CONTROL_FLAGS: [Flag<ControlFlags>; 8] = [
    Flag {
        name: "parenb",
        flag: ControlFlags::PARENB,
        show: true,
        sane: false,
    },
    Flag {
        name: "parodd",
        flag: ControlFlags::PARODD,
        show: true,
        sane: false,
    },
    Flag {
        name: "cmspar",
        flag: ControlFlags::CMSPAR,
        show: true,
        sane: false,
    },
    Flag {
        name: "hupcl",
        flag: ControlFlags::HUPCL,
        show: true,
        sane: true,
    },
    Flag {
        name: "cstopb",
        flag: ControlFlags::CSTOPB,
        show: true,
        sane: false,
    },
    Flag {
        name: "cread",
        flag: ControlFlags::CREAD,
        show: true,
        sane: true,
    },
    Flag {
        name: "clocal",
        flag: ControlFlags::CLOCAL,
        show: true,
        sane: false,
    },
    Flag {
        name: "crtscts",
        flag: ControlFlags::CRTSCTS,
        show: true,
        sane: false,
    },
];

pub const INPUT_FLAGS: [Flag<InputFlags>; 15] = [
    Flag {
        name: "ignbrk",
        flag: InputFlags::IGNBRK,
        show: true,
        sane: false,
    },
    Flag {
        name: "brkint",
        flag: InputFlags::BRKINT,
        show: true,
        sane: true,
    },
    Flag {
        name: "ignpar",
        flag: InputFlags::IGNPAR,
        show: true,
        sane: false,
    },
    Flag {
        name: "parmrk",
        flag: InputFlags::PARMRK,
        show: true,
        sane: false,
    },
    Flag {
        name: "inpck",
        flag: InputFlags::INPCK,
        show: true,
        sane: false,
    },
    Flag {
        name: "istrip",
        flag: InputFlags::ISTRIP,
        show: true,
        sane: false,
    },
    Flag {
        name: "inlcr",
        flag: InputFlags::INLCR,
        show: true,
        sane: false,
    },
    Flag {
        name: "igncr",
        flag: InputFlags::IGNCR,
        show: true,
        sane: false,
    },
    Flag {
        name: "icrnl",
        flag: InputFlags::ICRNL,
        show: true,
        sane: true,
    },
    Flag {
        name: "ixoff",
        flag: InputFlags::IXOFF,
        show: true,
        sane: false,
    },
    Flag {
        name: "tandem",
        flag: InputFlags::IXOFF,
        show: false,
        sane: false,
    },
    Flag {
        name: "ixon",
        flag: InputFlags::IXON,
        show: true,
        sane: false,
    },
    // not supported by nix
    // Flag {
    //     name: "iuclc",
    //     flag: InputFlags::IUCLC,
    //     show: true,
    //     default: false,
    // },
    Flag {
        name: "ixany",
        flag: InputFlags::IXANY,
        show: true,
        sane: false,
    },
    Flag {
        name: "imaxbel",
        flag: InputFlags::IMAXBEL,
        show: true,
        sane: true,
    },
    Flag {
        name: "iutf8",
        flag: InputFlags::IUTF8,
        show: true,
        sane: false,
    },
];

pub const OUTPUT_FLAGS: [Flag<OutputFlags>; 8] = [
    Flag {
        name: "opost",
        flag: OutputFlags::OPOST,
        show: true,
        sane: true,
    },
    Flag {
        name: "olcuc",
        flag: OutputFlags::OLCUC,
        show: true,
        sane: false,
    },
    Flag {
        name: "ocrnl",
        flag: OutputFlags::OCRNL,
        show: true,
        sane: false,
    },
    Flag {
        name: "onlcr",
        flag: OutputFlags::ONLCR,
        show: true,
        sane: true,
    },
    Flag {
        name: "onocr",
        flag: OutputFlags::ONOCR,
        show: true,
        sane: false,
    },
    Flag {
        name: "onlret",
        flag: OutputFlags::ONLRET,
        show: true,
        sane: false,
    },
    Flag {
        name: "ofill",
        flag: OutputFlags::OFILL,
        show: true,
        sane: false,
    },
    Flag {
        name: "ofdel",
        flag: OutputFlags::OFDEL,
        show: true,
        sane: false,
    },
];

pub const LOCAL_FLAGS: [Flag<LocalFlags>; 18] = [
    Flag {
        name: "isig",
        flag: LocalFlags::ISIG,
        show: true,
        sane: true,
    },
    Flag {
        name: "icanon",
        flag: LocalFlags::ICANON,
        show: true,
        sane: true,
    },
    Flag {
        name: "iexten",
        flag: LocalFlags::IEXTEN,
        show: true,
        sane: true,
    },
    Flag {
        name: "echo",
        flag: LocalFlags::ECHO,
        show: true,
        sane: true,
    },
    Flag {
        name: "echoe",
        flag: LocalFlags::ECHOE,
        show: true,
        sane: true,
    },
    Flag {
        name: "crterase",
        flag: LocalFlags::ECHOE,
        show: false,
        sane: true,
    },
    Flag {
        name: "echok",
        flag: LocalFlags::ECHOK,
        show: true,
        sane: true,
    },
    Flag {
        name: "echonl",
        flag: LocalFlags::ECHONL,
        show: true,
        sane: false,
    },
    Flag {
        name: "noflsh",
        flag: LocalFlags::NOFLSH,
        show: true,
        sane: false,
    },
    // Not supported by nix
    // Flag {
    //     name: "xcase",
    //     flag: LocalFlags::XCASE,
    //     show: true,
    //     sane: false,
    // },
    Flag {
        name: "tostop",
        flag: LocalFlags::TOSTOP,
        show: true,
        sane: false,
    },
    Flag {
        name: "echoprt",
        flag: LocalFlags::ECHOPRT,
        show: true,
        sane: false,
    },
    Flag {
        name: "prterase",
        flag: LocalFlags::ECHOPRT,
        show: false,
        sane: false,
    },
    Flag {
        name: "echoctl",
        flag: LocalFlags::ECHOCTL,
        show: true,
        sane: true,
    },
    Flag {
        name: "ctlecho",
        flag: LocalFlags::ECHOCTL,
        show: false,
        sane: true,
    },
    Flag {
        name: "echoke",
        flag: LocalFlags::ECHOKE,
        show: true,
        sane: true,
    },
    Flag {
        name: "crtkill",
        flag: LocalFlags::ECHOKE,
        show: false,
        sane: true,
    },
    Flag {
        name: "flusho",
        flag: LocalFlags::FLUSHO,
        show: true,
        sane: false,
    },
    Flag {
        name: "extproc",
        flag: LocalFlags::EXTPROC,
        show: true,
        sane: false,
    },
];
