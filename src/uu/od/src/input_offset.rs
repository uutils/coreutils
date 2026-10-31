// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fmt;
use std::io::{self, Write};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Radix {
    Decimal,
    Hexadecimal,
    Octal,
    NoPrefix,
}

/// provides the byte offset printed at the left margin
pub struct InputOffset {
    /// The radix to print the byte offset. [`Radix::NoPrefix`] will not print a byte offset.
    radix: Radix,
    /// The current position. Initialize at `new`, increase using `increase_position`.
    byte_pos: u64,
    /// An optional label printed in parentheses, typically different from `byte_pos`,
    /// but will increase with the same value if `byte_pos` in increased.
    label: Option<u64>,
}

impl InputOffset {
    /// creates a new `InputOffset` using the provided values.
    pub fn new(radix: Radix, byte_pos: u64, label: Option<u64>) -> Self {
        Self {
            radix,
            byte_pos,
            label,
        }
    }

    /// Increase `byte_pos` and `label` if a label is used.
    pub fn increase_position(&mut self, n: u64) {
        self.byte_pos += n;
        if let Some(l) = self.label {
            self.label = Some(l + n);
        }
    }

    #[cfg(test)]
    fn set_radix(&mut self, radix: Radix) {
        self.radix = radix;
    }

    /// returns a string with the current byte offset
    pub fn format_byte_offset(&self) -> String {
        self.to_string()
    }

    /// Prints the byte offset followed by a newline, or nothing at all if
    /// both `Radix::NoPrefix` was set and no label (--traditional) is used.
    pub fn write_final_offset(&self, writer: &mut impl Write) -> io::Result<()> {
        if self.radix != Radix::NoPrefix || self.label.is_some() {
            writeln!(writer, "{}", self.format_byte_offset())?;
        }
        Ok(())
    }
}

/// Writes the current byte offset, as [`InputOffset::format_byte_offset`] returns it.
impl fmt::Display for InputOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.radix, self.label) {
            (Radix::Decimal, None) => write!(f, "{:07}", self.byte_pos),
            (Radix::Decimal, Some(l)) => write!(f, "{:07} ({l:07})", self.byte_pos),
            (Radix::Hexadecimal, None) => write!(f, "{:06x}", self.byte_pos),
            (Radix::Hexadecimal, Some(l)) => write!(f, "{:06x} ({l:06x})", self.byte_pos),
            (Radix::Octal, None) => write!(f, "{:07o}", self.byte_pos),
            (Radix::Octal, Some(l)) => write!(f, "{:07o} ({l:07o})", self.byte_pos),
            (Radix::NoPrefix, None) => Ok(()),
            (Radix::NoPrefix, Some(l)) => write!(f, "({l:07o})"),
        }
    }
}

#[test]
fn test_input_offset() {
    let mut sut = InputOffset::new(Radix::Hexadecimal, 10, None);
    assert_eq!("00000a", &sut.format_byte_offset());
    sut.increase_position(10);
    assert_eq!("000014", &sut.format_byte_offset());

    // note normally the radix will not change after initialization
    sut.set_radix(Radix::Decimal);
    assert_eq!("0000020", &sut.format_byte_offset());

    sut.set_radix(Radix::Hexadecimal);
    assert_eq!("000014", &sut.format_byte_offset());

    sut.set_radix(Radix::Octal);
    assert_eq!("0000024", &sut.format_byte_offset());

    sut.set_radix(Radix::NoPrefix);
    assert_eq!("", &sut.format_byte_offset());

    sut.increase_position(10);
    sut.set_radix(Radix::Octal);
    assert_eq!("0000036", &sut.format_byte_offset());
}

#[test]
fn test_input_offset_with_label() {
    let mut sut = InputOffset::new(Radix::Hexadecimal, 10, Some(20));
    assert_eq!("00000a (000014)", &sut.format_byte_offset());
    sut.increase_position(10);
    assert_eq!("000014 (00001e)", &sut.format_byte_offset());

    // note normally the radix will not change after initialization
    sut.set_radix(Radix::Decimal);
    assert_eq!("0000020 (0000030)", &sut.format_byte_offset());

    sut.set_radix(Radix::Hexadecimal);
    assert_eq!("000014 (00001e)", &sut.format_byte_offset());

    sut.set_radix(Radix::Octal);
    assert_eq!("0000024 (0000036)", &sut.format_byte_offset());

    sut.set_radix(Radix::NoPrefix);
    assert_eq!("(0000036)", &sut.format_byte_offset());

    sut.increase_position(10);
    sut.set_radix(Radix::Octal);
    assert_eq!("0000036 (0000050)", &sut.format_byte_offset());
}
