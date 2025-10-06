// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::{EscapeState, EscapedChar, Quoter, Quotes};

// These are characters with special meaning in the shell (e.g. bash). The
// first const contains characters that only have a special meaning when they
// appear at the beginning of a name.
const SPECIAL_SHELL_CHARS_START: &[u8] = b"~#";

// Escaped and NonEscaped shell quoting strategies are very different.
// Therefore, we are using separate Quoter structures for each of them.

pub(super) struct NonEscapedShellQuoter<'a> {
    // INIT
    /// Original name.
    reference: &'a [u8],

    /// The quotes to be used if necessary
    quotes: Quotes,

    /// Whether to show control and non-unicode characters, or replace them
    /// with `?`.
    show_control: bool,

    // INTERNAL STATE
    /// Whether the name should be quoted.
    must_quote: bool,

    buffer: Vec<u8>,
}

impl<'a> NonEscapedShellQuoter<'a> {
    pub fn new(
        reference: &'a [u8],
        show_control: bool,
        always_quote: bool,
        dirname: bool,
        size_hint: usize,
    ) -> Self {
        let (quotes, must_quote) = initial_quoting(reference, dirname, always_quote);
        Self {
            reference,
            quotes,
            show_control,
            must_quote,
            buffer: Vec::with_capacity(size_hint),
        }
    }
}

impl Quoter for NonEscapedShellQuoter<'_> {
    fn push_char(&mut self, input: char) {
        let escaped = EscapedChar::new_shell(input, false, self.quotes);

        let escaped = if self.show_control {
            escaped
        } else {
            escaped.hide_control()
        };

        match escaped.state {
            EscapeState::Backslash('\'') => self.buffer.extend(b"'\\''"),
            EscapeState::ForceQuote(x) => {
                self.must_quote = true;
                self.buffer.extend(x.to_string().as_bytes());
            }
            _ => {
                self.buffer.extend(escaped.collect::<String>().as_bytes());
            }
        }
    }

    fn push_invalid(&mut self, input: &[u8]) {
        if self.show_control {
            self.buffer.extend(input);
        } else {
            self.buffer.extend(std::iter::repeat_n(b'?', input.len()));
        }
    }

    fn finalize(self: Box<Self>) -> Vec<u8> {
        finalize_shell_quoter(self.buffer, self.reference, self.must_quote, self.quotes)
    }
}

// We need to keep track of whether we are in a dollar expression
// because e.g. \b\n is escaped as $'\b\n' and not like $'b'$'n'
pub(super) struct EscapedShellQuoter<'a> {
    // INIT
    /// Original name.
    reference: &'a [u8],

    /// The quotes to be used if necessary
    quotes: Quotes,

    // INTERNAL STATE
    /// Whether the name should be quoted.
    must_quote: bool,

    /// Whether we are currently in a dollar escaped environment.
    in_dollar: bool,

    buffer: Vec<u8>,
}

impl<'a> EscapedShellQuoter<'a> {
    pub fn new(reference: &'a [u8], always_quote: bool, dirname: bool, size_hint: usize) -> Self {
        let (quotes, must_quote) = initial_quoting(reference, dirname, always_quote);
        Self {
            reference,
            quotes,
            must_quote,
            in_dollar: false,
            buffer: Vec::with_capacity(size_hint),
        }
    }

    fn enter_dollar(&mut self) {
        if !self.in_dollar {
            self.buffer.extend(b"'$'");
            self.in_dollar = true;
        }
    }

    fn exit_dollar(&mut self) {
        if self.in_dollar {
            self.buffer.extend(b"''");
            self.in_dollar = false;
        }
    }
}

impl Quoter for EscapedShellQuoter<'_> {
    fn push_char(&mut self, input: char) {
        let escaped = EscapedChar::new_shell(input, true, self.quotes);
        match escaped.state {
            EscapeState::Char(x) => {
                self.exit_dollar();
                self.buffer.extend(x.to_string().as_bytes());
            }
            EscapeState::ForceQuote(x) => {
                self.exit_dollar();
                self.must_quote = true;
                self.buffer.extend(x.to_string().as_bytes());
            }
            // Single quotes are not put in dollar expressions, but are escaped
            // if the string also contains double quotes. In that case, they
            // must be handled separately.
            EscapeState::Backslash('\'') => {
                self.must_quote = true;
                self.in_dollar = false;
                self.buffer.extend(b"'\\''");
            }
            _ => {
                self.enter_dollar();
                self.must_quote = true;
                self.buffer.extend(escaped.collect::<String>().as_bytes());
            }
        }
    }

    fn push_invalid(&mut self, input: &[u8]) {
        // Early return on empty inputs.
        if input.is_empty() {
            return;
        }

        self.enter_dollar();
        self.must_quote = true;
        self.buffer.extend(
            input
                .iter()
                .flat_map(|b| EscapedChar::new_octal(*b))
                .collect::<String>()
                .as_bytes(),
        );
    }

    fn finalize(self: Box<Self>) -> Vec<u8> {
        finalize_shell_quoter(self.buffer, self.reference, self.must_quote, self.quotes)
    }
}

/// Deduce the initial quoting status from the provided information
fn initial_quoting(input: &[u8], dirname: bool, always_quote: bool) -> (Quotes, bool) {
    if input
        .iter()
        .any(|c| shell_escaped_char_set(dirname).contains(c))
    {
        (Quotes::Single, true)
    } else if input.contains(&b'\'') {
        (Quotes::Double, true)
    } else if always_quote || input.is_empty() {
        (Quotes::Single, true)
    } else {
        (Quotes::Single, false)
    }
}

/// Check whether `bytes` starts with any byte in `pattern`.
fn bytes_start_with(bytes: &[u8], pattern: &[u8]) -> bool {
    !bytes.is_empty() && pattern.contains(&bytes[0])
}

/// Return a set of characters that implies quoting of the word in
/// shell-quoting mode.
fn shell_escaped_char_set(is_dirname: bool) -> &'static [u8] {
    const ESCAPED_CHARS: &[u8] = b":\"`$\\^\n\t\r=";
    // the ':' colon character only induce quoting in the
    // context of ls displaying a directory name before listing its content.
    // (e.g. with the recursive flag -R)
    let start_index = usize::from(!is_dirname);
    &ESCAPED_CHARS[start_index..]
}

fn finalize_shell_quoter(
    buffer: Vec<u8>,
    reference: &[u8],
    must_quote: bool,
    quotes: Quotes,
) -> Vec<u8> {
    let contains_quote_chars = must_quote || bytes_start_with(reference, SPECIAL_SHELL_CHARS_START);

    if must_quote | contains_quote_chars && quotes != Quotes::None {
        let mut quoted = Vec::<u8>::with_capacity(buffer.len() + 2);
        let quote = if quotes == Quotes::Single {
            b'\''
        } else {
            b'"'
        };
        quoted.push(quote);
        quoted.extend(buffer);
        quoted.push(quote);
        quoted
    } else {
        buffer
    }
}
