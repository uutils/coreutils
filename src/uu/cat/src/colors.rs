use std::env::var_os;
use std::f64::consts::PI;
use std::ffi::OsStr;
use std::io::{Result as IOResult, Write};

use crate::OutputState;

static TRUECOLOR_ESCAPE_START: &str = "\x1b[38;2";
static ANSI_ESCAPE_START: &str = "\x1b[38;5;";

static ANSI_PALETTE: [[f64; 3]; 12] = [
    // regular
    [128., 0., 0.],
    [0., 128., 0.],
    [128., 128., 0.],
    [0., 0., 128.],
    [128., 0., 128.],
    [0., 128., 128.],
    // bright
    [255., 0., 0.],
    [0., 255., 0.],
    [255., 255., 0.],
    [0., 0., 255.],
    [255., 0., 255.],
    [0., 255., 255.],
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
    Ansi,
}

impl ColorMode {
    fn conv_color(&self, color: [f64; 3]) -> Color {
        let fit_linear_curve = |x: f64| 127. * x + 128.;
        match self {
            ColorMode::TrueColor => Color::Color24b(color.map(|x| fit_linear_curve(x) as u8)),
            ColorMode::Ansi256 => {
                let ratio = [36, 6, 1];
                let ascii_color_offset = 16u16;
                Color::Ansi256(
                    color
                        .into_iter()
                        .map(fit_linear_curve)
                        .zip(ratio)
                        .fold(ascii_color_offset, |acc, (c, m)| {
                            acc + ((6. * (c / 256.)).floor() as u16) * m
                        }),
                )
            }
            ColorMode::Ansi => {
                let [ansi_normal_radix, ansi_bright_radix] = [31, 91];
                let taxi_cab_distance = |b: [f64; 3]| {
                    color
                        .into_iter()
                        .map(fit_linear_curve)
                        .zip(b)
                        .fold(0., |acc, (a, b)| acc + (a - b).abs())
                };
                let closest_match = ANSI_PALETTE
                    .into_iter()
                    .map(taxi_cab_distance)
                    .enumerate()
                    .reduce(|m @ (_, min), e @ (_, n)| if min <= n { m } else { e })
                    .map_or(0, |(i, _)| i as u8);
                Color::Ansi(if closest_match < 6 {
                    ansi_normal_radix + closest_match
                } else {
                    ansi_bright_radix + (closest_match - 6)
                })
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Color {
    Color24b([u8; 3]),
    Ansi256(u16),
    Ansi(u8),
}

impl Color {
    fn format_char(&self, buf: &mut Vec<u8>, ch: u8) {
        // avoid rust format machinery
        let format_num = |b: &mut Vec<u8>, mut n| {
            let ascii_num_radix = 48;
            let buf_len = b.len();
            if n == 0 {
                b.push(ascii_num_radix);
                return;
            }
            while n > 0 {
                b.push(ascii_num_radix + (n % 10) as u8);
                n /= 10;
            }
            b[buf_len..].reverse();
        };

        // format according to escape sequence needed
        match *self {
            Color::Color24b(color) => {
                buf.truncate(TRUECOLOR_ESCAPE_START.len());
                for c in color {
                    buf.push(b';');
                    format_num(buf, c as usize);
                }
                buf.push(b'm');
                buf.push(ch);
            }
            Color::Ansi256(color) => {
                buf.truncate(ANSI_ESCAPE_START.len());
                format_num(buf, color as usize);
                buf.push(b'm');
                buf.push(ch);
            }
            Color::Ansi(color) => {
                buf.truncate(ANSI_ESCAPE_START.len());
                format_num(buf, color as usize);
                buf.push(b'm');
                buf.push(ch);
            }
        }
    }
}

impl ColorMode {
    pub fn new() -> Self {
        if cfg!(target_os = "windows")
            || var_os("COLORTERM")
                .is_some_and(|ref x| x == OsStr::new("truecolor") || x == OsStr::new("24bit"))
            || var_os("CI").is_some()
            || var_os("WSL_DISTRO_NAME").is_some()
        {
            ColorMode::TrueColor
        } else if var_os("TERM").is_some_and(|ref x| x == OsStr::new("xterm-256color")) {
            ColorMode::Ansi256
        } else {
            ColorMode::Ansi
        }
    }
}

/// This is a wrapper over a a Writer that
/// intersperses color escape codes in between
/// written characters, assuming no line breaks.
pub struct ColorWriter<'w, W: Write> {
    inner: &'w mut W,
    mode: ColorMode,
    buffer: Vec<u8>,
    state: &'w mut OutputState,
    terminal_cursor: usize,
}

impl<'w, W: Write> ColorWriter<'w, W> {
    const COLOR_DIFFUSION: f64 = 0.06;

    pub fn new(inner: &'w mut W, mode: ColorMode, state: &'w mut OutputState) -> Self {
        Self {
            inner,
            mode,
            buffer: Vec::from(match mode {
                ColorMode::TrueColor => TRUECOLOR_ESCAPE_START,
                _ => ANSI_ESCAPE_START,
            }),
            state,
            terminal_cursor: 0,
        }
    }

    /// Compute color in sinus-image range; heavily based on
    /// <https://github.com/ur0/lolcat>; MIT-licensed and
    /// co-copyright of Umang Raghuvanshi et al.
    fn get_color(&self) -> Color {
        let color = Self::COLOR_DIFFUSION * self.state.color_seed[0];
        let two_thirds = 2. / 3.;
        let rgb = [
            color,
            color + (PI * two_thirds),
            color + (PI * 2. * two_thirds),
        ]
        .map(f64::sin);

        self.mode.conv_color(rgb)
    }

    fn next_col(&mut self) {
        self.state.color_seed[0] += 1.;
    }

    fn next_row(&mut self) {
        self.state.color_seed[1] += 1.;
        self.state.color_seed[0] = self.state.color_seed[1];
        self.terminal_cursor = 0;
    }

    /// Along with the color gradient algorithm,
    /// this escape sequence parser is heavily based
    /// on <https://github.com/ur0/lolcat>; MIT-licensed
    /// and co-copyright of Umang Raghuvanshi et al.
    // Beware of the following spaghetti.
    fn parse_escape_seq<'a, 'b>(chars: &'a mut impl Iterator<Item = &'b u8>) -> Result<String, ()> {
        let mut buf = String::with_capacity(16);
        buf.push('\x1b');

        let mut next_ch = || {
            let Some(ch) = chars.next().map(|x| *x as char) else {
                return Err(());
            };
            buf.push(ch);
            Ok(ch)
        };

        match next_ch()? {
            '[' => 'l1: loop {
                match next_ch()? {
                    '\x30'..='\x3F' => continue 'l1,
                    '\x20'..='\x2F' => {
                        'l2: loop {
                            match next_ch()? {
                                '\x20'..='\x2F' => continue 'l2,
                                '\x40'..='\x7E' => break 'l2,
                                _ => return Err(()),
                            }
                        }
                        break 'l1;
                    }
                    '\x40'..='\x7E' => break 'l1,
                    _ => return Err(()),
                }
            },
            '\x20'..='\x2F' => 'l2: loop {
                match next_ch()? {
                    '\x20'..='\x2F' => continue 'l2,
                    '\x30'..='\x7E' => break 'l2,
                    _ => return Err(()),
                }
            },
            // Unsupported, obscure escape sequences
            '\x30'..='\x3F' | '\x40'..='\x5F' | '\x60'..='\x7E' => return Err(()),
            // Assume the sequence is just one character otherwise
            _ => (),
        }
        Ok(buf)
    }
}

impl<W: Write> Write for ColorWriter<'_, W> {
    fn flush(&mut self) -> IOResult<()> {
        // reset colors after flush
        self.inner.write_all(b"\x1b[39m")?;
        self.inner.flush()
    }

    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        let mut chars = buf.iter();

        while let Some(&ch) = chars.next() {
            match ch {
                // relay escape sequences verbatim
                b'\x1b' => {
                    let Ok(buf) = Self::parse_escape_seq(&mut chars) else {
                        continue;
                    };
                    self.inner.write_all(buf.as_bytes())?;
                }
                // skip colorization of whitespaces or tabs
                c @ (b'\x20' | b'\x09' | b'\x0b') => {
                    self.inner.write(&[c])?;
                }
                // If not an escape sequence or a newline, print the
                // color escape sequence and then the character
                _ => {
                    self.get_color().format_char(&mut self.buffer, ch);
                    self.inner.write_all(&self.buffer)?;
                }
            }
            self.next_col();
        }
        self.next_row();
        self.inner.write_all(b"\x1b[39m").map(|_| buf.len())
    }
}
