//! Provides consistent newline/zero terminator handling
use std::fmt::Display;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Line terminator used for printing and parsing
pub enum LineEnding {
    #[default] Newline = b'\n',
    Nul = 0,
    Space = b' ',
    None = 8, // abuse backspace \b to encode None
}

impl Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Newline => writeln!(f),
            Self::Nul => write!(f, "\0"),
            Self::Space => write!(f, " "),
            Self::None => std::fmt::Result::Ok(()),
        }
    }
}

impl From<LineEnding> for u8 {
    fn from(line_ending: LineEnding) -> Self {
        match line_ending {
            LineEnding::None => panic!("Cannot convert LineEnding::None to u8"),
            _ => line_ending as Self,
        }
    }
}

impl From<bool> for LineEnding {
    fn from(is_zero_terminated: bool) -> Self {
        if is_zero_terminated {
            Self::Nul
        } else {
            Self::Newline
        }
    }
}
