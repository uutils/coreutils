// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Provides consistent newline/zero terminator handling for `-z`/`--zero` flags.
//!
//! See the [`LineEnding`] struct for more information.
use std::fmt::Display;

/// Line ending of either `\n` or `\0`
///
/// Used by various utilities that have the option to separate lines by nul
/// characters instead of `\n`. Usually, this is specified with the `-z` or
/// `--zero` flag.
///
/// The [`Display`] implementation writes the character corresponding to the
/// variant to the formatter.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LineEnding {
    #[default]
    /// Newline character (`\n`)
    Newline = b'\n',

    /// Null character (`\0`)
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
    /// Create a [`LineEnding`] from a `-z`/`--zero` flag
    ///
    /// If `is_zero_terminated` is true, [`LineEnding::Nul`] is returned,
    /// otherwise [`LineEnding::Newline`].
    pub fn from_zero_flag(is_zero_terminated: bool) -> Self {
        if is_zero_terminated {
            Self::Nul
        } else {
            Self::Newline
        }
    }
}
