//! Provides consistent newline/zero terminator handling
use std::fmt::Display;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Line terminator used for printing and parsing
pub enum LineEnding {
    #[default]
    Newline = b'\n',
    Nul = 0,
}

impl Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Newline => writeln!(f),
            Self::Nul => write!(f, "\0"),
        }
    }
}

impl From<LineEnding> for u8 {
    fn from(line_ending: LineEnding) -> Self {
        line_ending as Self
    }
}

impl LineEnding {
    pub fn from_zero_flag(is_zero_terminated: bool) -> Self {
        if is_zero_terminated {
            Self::Nul
        } else {
            Self::Newline
        }
    }
}
