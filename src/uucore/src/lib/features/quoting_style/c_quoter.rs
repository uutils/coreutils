use super::{EscapedChar, Quoter, Quotes};

pub(super) struct CQuoter {
    /// The type of quotes to use.
    quotes: Quotes,

    dirname: bool,

    buffer: Vec<u8>,
}

impl CQuoter {
    pub fn new(quotes: Quotes, dirname: bool, size_hint: usize) -> Self {
        let mut buffer = Vec::with_capacity(size_hint);
        match quotes {
            Quotes::None => (),
            Quotes::Single => buffer.push(b'\''),
            Quotes::Double => buffer.push(b'"'),
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

    fn finalize(mut self: Box<Self>) -> Vec<u8> {
        match self.quotes {
            Quotes::None => (),
            Quotes::Single => self.buffer.push(b'\''),
            Quotes::Double => self.buffer.push(b'"'),
        }
        self.buffer
    }
}
