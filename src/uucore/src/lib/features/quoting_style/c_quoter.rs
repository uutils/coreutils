// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::{EscapedChar, Quoter, Quotes, locale_quotes};

pub(super) struct CQuoter {
    /// The type of quotes to use.
    quotes: Quotes,

    /// Opening quote character (for Locale variant).
    #[allow(dead_code)] // Used in new() to initialize buffer
    open_quote: char,

    /// Closing quote character (for Locale variant).
    close_quote: char,

    dirname: bool,

    buffer: Vec<u8>,
}

impl CQuoter {
    pub fn new(quotes: Quotes, dirname: bool, size_hint: usize) -> Self {
        let mut buffer = Vec::with_capacity(size_hint);

        // Determine quote characters based on quote type
        let (open_quote, close_quote) = match quotes {
            Quotes::None => ('\0', '\0'),
            Quotes::Single => ('\'', '\''),
            Quotes::Double => ('"', '"'),
            Quotes::Locale => locale_quotes::get_locale_quote_chars(),
        };

        // Add opening quote to buffer
        match quotes {
            Quotes::None => (),
            Quotes::Single | Quotes::Double => buffer.push(open_quote as u8),
            Quotes::Locale => {
                // Push UTF-8 encoded quote character
                let mut buf = [0; 4];
                let quote_str = open_quote.encode_utf8(&mut buf);
                buffer.extend_from_slice(quote_str.as_bytes());
            }
        }

        Self {
            quotes,
            open_quote,
            close_quote,
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

    fn finalize(mut self: Box<Self>) -> Vec<u8> {
        // Add closing quote to buffer
        match self.quotes {
            Quotes::None => (),
            Quotes::Single | Quotes::Double => self.buffer.push(self.close_quote as u8),
            Quotes::Locale => {
                // Push UTF-8 encoded closing quote character
                let mut buf = [0; 4];
                let quote_str = self.close_quote.encode_utf8(&mut buf);
                self.buffer.extend_from_slice(quote_str.as_bytes());
            }
        }
        self.buffer
    }
}
