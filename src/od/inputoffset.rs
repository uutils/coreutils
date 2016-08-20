
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Radix { Decimal, Hexadecimal, Octal, NoPrefix }

/// provides the byte offset printed at the left margin
pub struct InputOffset {
    /// The radix to print the byte offset. NoPrefix will not print a byte offset.
    radix: Radix,
    /// The current position. Initialize at `new`, increase using `increase_position`.
    byte_pos: usize,
    /// An optional label printed in parentheses, typically different from `byte_pos`,
    /// but will increase with the same value if `byte_pos` in increased.
    label: Option<usize>,
}

impl InputOffset {
    /// creates a new `InputOffset` using the provided values.
    pub fn new(radix: Radix, byte_pos: usize, label: Option<usize>) -> InputOffset {
        InputOffset {
            radix: radix,
            byte_pos: byte_pos,
            label: label,
        }
    }

    /// Increase `byte_pos` and `label` if a label is used.
    pub fn increase_position(&mut self, n: usize) {
        self.byte_pos += n;
        if let Some(l) = self.label {
            self.label = Some(l + n);
        }
    }

    /// set `self.radix` to the value provided by the --address-radix commandline option
    pub fn parse_radix_from_commandline(&mut self, radix_str: Option<String>) -> Result<(), &'static str> {
        match radix_str {
            None => self.radix = Radix::Octal,
            Some(s) => {
                let st = s.into_bytes();
                if st.len() != 1 {
                    return Err("Radix must be one of [d, o, n, x]\n")
                } else {
                    let radix: char = *(st.get(0)
                                          .expect("byte string of length 1 lacks a 0th elem")) as char;
                    match radix {
                        'd' => self.radix = Radix::Decimal,
                        'x' => self.radix = Radix::Hexadecimal,
                        'o' => self.radix = Radix::Octal,
                        'n' => self.radix = Radix::NoPrefix,
                        _ => return Err("Radix must be one of [d, o, n, x]\n")
                    }
                }
            }
        }
        Ok(())
    }

    /// returns a string with the current byte offset
    pub fn format_byte_offset(&self) -> String {
        match (self.radix, self.label) {
            (Radix::Decimal, None) => format!("{:07}", self.byte_pos),
            (Radix::Decimal, Some(l)) => format!("{:07} ({:07})", self.byte_pos, l),
            (Radix::Hexadecimal, None) => format!("{:06X}", self.byte_pos),
            (Radix::Hexadecimal, Some(l)) => format!("{:06X} ({:06X})", self.byte_pos, l),
            (Radix::Octal, None) => format!("{:07o}", self.byte_pos),
            (Radix::Octal, Some(l)) => format!("{:07o} ({:07o})", self.byte_pos, l),
            (Radix::NoPrefix, None) => String::from(""),
            (Radix::NoPrefix, Some(l)) => format!("({:07o})", l),
        }
    }

    /// Prints the byte offset followed by a newline, or nothing at all if
    /// both `Radix::NoPrefix` was set and no label (--traditional) is used.
    pub fn print_final_offset(&self) {
        if self.radix != Radix::NoPrefix || self.label.is_some() {
            print!("{}\n", self.format_byte_offset());
        }
    }
}

#[test]
fn test_input_offset() {
    let mut sut = InputOffset::new(Radix::Hexadecimal, 10, None);
    assert_eq!("00000A", &sut.format_byte_offset());
    sut.increase_position(10);
    assert_eq!("000014", &sut.format_byte_offset());

    // note normally the radix will not change after initialisation
    sut.parse_radix_from_commandline(Some("d".to_string())).unwrap();
    assert_eq!("0000020", &sut.format_byte_offset());

    sut.parse_radix_from_commandline(Some("x".to_string())).unwrap();
    assert_eq!("000014", &sut.format_byte_offset());

    sut.parse_radix_from_commandline(Some("o".to_string())).unwrap();
    assert_eq!("0000024", &sut.format_byte_offset());

    sut.parse_radix_from_commandline(Some("n".to_string())).unwrap();
    assert_eq!("", &sut.format_byte_offset());

    sut.increase_position(10);
    sut.parse_radix_from_commandline(None).unwrap();
    assert_eq!("0000036", &sut.format_byte_offset());
}

#[test]
fn test_input_offset_with_label() {
    let mut sut = InputOffset::new(Radix::Hexadecimal, 10, Some(20));
    assert_eq!("00000A (000014)", &sut.format_byte_offset());
    sut.increase_position(10);
    assert_eq!("000014 (00001E)", &sut.format_byte_offset());

    // note normally the radix will not change after initialisation
    sut.parse_radix_from_commandline(Some("d".to_string())).unwrap();
    assert_eq!("0000020 (0000030)", &sut.format_byte_offset());

    sut.parse_radix_from_commandline(Some("x".to_string())).unwrap();
    assert_eq!("000014 (00001E)", &sut.format_byte_offset());

    sut.parse_radix_from_commandline(Some("o".to_string())).unwrap();
    assert_eq!("0000024 (0000036)", &sut.format_byte_offset());

    sut.parse_radix_from_commandline(Some("n".to_string())).unwrap();
    assert_eq!("(0000036)", &sut.format_byte_offset());

    sut.increase_position(10);
    sut.parse_radix_from_commandline(None).unwrap();
    assert_eq!("0000036 (0000050)", &sut.format_byte_offset());
}
