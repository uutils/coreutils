// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::i18n::UEncoding;

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

    #[inline]
    fn size_hint(self) -> usize {
        self.opening.len_utf8() + self.closing.len_utf8()
    }

    /// Return [`Self::LOCALE_UTF8`] if `encoding` is [`UEncoding::Utf8`],
    /// otherwise return `other`.
    #[inline]
    pub(super) fn utf8_or(encoding: UEncoding, other: Self) -> Self {
        match encoding {
            UEncoding::Utf8 => Self::LOCALE_UTF8,
            UEncoding::Ascii => other,
        }
    }

    /// Wrap `self` in [`CQuotesEnforce::Always`]
    #[inline]
    pub(super) fn always(self) -> CQuotesEnforce {
        CQuotesEnforce::Always(self)
    }

    /// Wrap `self` in [`CQuotesEnforce::Always`] if `always_quote` is
    /// true, else wrap it in [`CQuotesEnforce::Maybe`].
    #[inline]
    pub(super) fn always_if(self, always_quotes: bool) -> CQuotesEnforce {
        if always_quotes {
            CQuotesEnforce::Always(self)
        } else {
            CQuotesEnforce::Maybe(self)
        }
    }

    pub(super) fn opening_as_utf8(self, buf: &mut [u8]) -> &[u8] {
        self.opening.encode_utf8(buf).as_bytes()
    }

    pub(super) fn closing_as_utf8(self, buf: &mut [u8]) -> &[u8] {
        self.closing.encode_utf8(buf).as_bytes()
    }
}

/// Decides whether the given quotes MUST be printed in all cases.
#[derive(Debug, Clone, Copy)]
pub(super) enum CQuotesEnforce {
    /// Always add surrounding quotes.
    Always(CQuotes),

    /// Only add quotes when at least one character was escaped.
    Maybe(CQuotes),

    /// No quotes.
    None,
}

impl CQuotesEnforce {
    fn as_cquotes(self) -> Option<CQuotes> {
        match self {
            Self::Always(cquotes) | Self::Maybe(cquotes) => Some(cquotes),
            Self::None => None,
        }
    }
}

/// A quoter to perform C-like quoting.
///
/// Used for quoting-styles:
/// - c
/// - c-maybe
/// - escape
/// - clocale
/// - locale
pub(super) struct CQuoter {
    /// The type of quotes to use, if any.
    quotes: CQuotesEnforce,

    /// Set upon encountering escaped characters.
    ///
    /// If true and `quotes` is [`CQuotesEnforce::Maybe`], use quotes.
    must_quote: bool,

    /// Passed to EscapeChar, changes the set of escaped characters.
    dirname: bool,

    buffer: Vec<u8>,
}

impl CQuoter {
    pub(super) fn new(quotes: CQuotesEnforce, dirname: bool, size_hint: usize) -> Self {
        let mut buffer = Vec::with_capacity(size_hint);

        if let CQuotesEnforce::Always(quotes) = quotes {
            let mut quote_buf = [0; 4];
            buffer.extend_from_slice(quotes.opening_as_utf8(&mut quote_buf));
        }

        Self {
            quotes,
            must_quote: false,
            dirname,
            buffer,
        }
    }
}

impl Quoter for CQuoter {
    fn push_char(&mut self, input: char) {
        let escaped =
            EscapedChar::new_c(input, self.quotes.as_cquotes(), self.dirname).hide_control();
        if escaped.is_escaping() {
            self.must_quote = true;
        }
        self.buffer.extend(escaped.collect::<String>().into_bytes());
    }

    fn push_invalid(&mut self, input: &[u8]) {
        for b in input {
            let escaped: String = EscapedChar::new_octal(*b).hide_control().collect();
            self.buffer.extend_from_slice(escaped.as_bytes());
        }
        if !input.is_empty() {
            self.must_quote = true;
        }
    }

    fn finalize(self: Box<Self>) -> Vec<u8> {
        use CQuotesEnforce::{Always, Maybe};

        let Self {
            quotes,
            must_quote,
            mut buffer,
            ..
        } = *self;
        let mut quote_buf = [0; 4];

        match (quotes, must_quote) {
            (Always(quotes), _) => {
                buffer.extend_from_slice(quotes.closing_as_utf8(&mut quote_buf));
            }
            (Maybe(quotes), true) => {
                // Need to insert the opening quote at the beginning
                let mut tmp = Vec::with_capacity(buffer.len() + quotes.size_hint());
                tmp.extend_from_slice(quotes.opening_as_utf8(&mut quote_buf));
                tmp.extend_from_slice(&buffer);
                tmp.extend_from_slice(quotes.closing_as_utf8(&mut quote_buf));

                buffer = tmp;
            }
            _ => {}
        }

        buffer
    }
}
