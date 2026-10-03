// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::{EscapedChar, Quoter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CQuotes {
    pub(super) opening: char,
    pub(super) closing: char,
}

impl CQuotes {
    pub const SINGLE: Self = Self {
        opening: '\'',
        closing: '\'',
    };

    pub const DOUBLE: Self = Self {
        opening: '"',
        closing: '"',
    };

    pub const LOCALE_UTF8: Self = Self {
        opening: '\u{2018}',
        closing: '\u{2019}',
    };

    pub(super) fn opening_as_utf8(self, buf: &mut [u8]) -> &[u8] {
        self.opening.encode_utf8(buf).as_bytes()
    }

    pub(super) fn closing_as_utf8(self, buf: &mut [u8]) -> &[u8] {
        self.closing.encode_utf8(buf).as_bytes()
    }
}

pub(super) struct CQuoter {
    /// The type of quotes to use, if any.
    quotes: Option<CQuotes>,

    dirname: bool,

    buffer: Vec<u8>,
}

impl CQuoter {
    pub(super) fn new(quotes: Option<CQuotes>, dirname: bool, size_hint: usize) -> Self {
        let mut buffer = Vec::with_capacity(size_hint);

        if let Some(quotes) = quotes {
            let mut quote_buf = [0; 4];
            buffer.extend_from_slice(quotes.opening_as_utf8(&mut quote_buf));
        }

        Self {
            quotes,
            dirname,
            buffer,
        }
    }
}

impl Quoter for CQuoter {
    fn push_char(&mut self, input: char) {
        let escaped: String = EscapedChar::new_c(input, self.quotes, self.dirname)
            .hide_control()
            .collect();
        self.buffer.extend_from_slice(escaped.as_bytes());
    }

    fn push_invalid(&mut self, input: &[u8]) {
        for b in input {
            let escaped: String = EscapedChar::new_octal(*b).hide_control().collect();
            self.buffer.extend_from_slice(escaped.as_bytes());
        }
    }

    fn finalize(self: Box<Self>) -> Vec<u8> {
        let Self {
            quotes, mut buffer, ..
        } = *self;
        if let Some(quotes) = quotes {
            let mut quote_buf = [0; 4];
            buffer.extend_from_slice(quotes.closing_as_utf8(&mut quote_buf));
        }

        buffer
    }
}
