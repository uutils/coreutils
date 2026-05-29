// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::{EscapedChar, Quoter};

pub(super) struct LiteralQuoter(Vec<u8>);

impl LiteralQuoter {
    pub fn new(size_hint: usize) -> Self {
        Self(Vec::with_capacity(size_hint))
    }
}

impl Quoter for LiteralQuoter {
    fn push_char(&mut self, input: char) {
        let escaped = EscapedChar::new_literal(input)
            .hide_control()
            .collect::<String>();
        self.0.extend(escaped.as_bytes());
    }

    fn push_invalid(&mut self, input: &[u8]) {
        self.0.extend(std::iter::repeat_n(b'?', input.len()));
    }

    fn finalize(self: Box<Self>) -> Vec<u8> {
        self.0
    }
}
