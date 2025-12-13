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

    /// Whether to always quote the output
    always_quote: bool,

    /// Whether to commit to dollar quoting for the entire string when control chars present
    commit_dollar: bool,

    // INTERNAL STATE
    /// Whether the name should be quoted.
    must_quote: bool,

    /// Whether we are currently in a dollar escaped environment.
    in_dollar: bool,

    /// Track if we're in an open quote section that needs closing
    in_quote_section: bool,

    buffer: Vec<u8>,
}

impl<'a> EscapedShellQuoter<'a> {
    pub fn new(
        reference: &'a [u8],
        always_quote: bool,
        dirname: bool,
        commit_dollar_mode: bool,
        size_hint: usize,
    ) -> Self {
        let (quotes, must_quote) = initial_quoting(reference, dirname, always_quote);

        // commit_dollar_mode controls quoting strategy:
        // true (printf %q): committed dollar mode - wrap entire string in $'...' when control chars present
        // false (ls): selective dollar mode - only wrap individual control chars in $'...'
        let commit_dollar = commit_dollar_mode;

        // Pre-scan for control chars if we're in committed mode
        let has_control_chars = commit_dollar && reference.iter().any(|&b| b.is_ascii_control());

        let mut buffer = Vec::with_capacity(size_hint);
        if has_control_chars {
            buffer.extend(b"$'");
        }

        Self {
            reference,
            quotes,
            always_quote,
            commit_dollar,
            must_quote,
            in_dollar: has_control_chars,
            in_quote_section: false,
            buffer,
        }
    }

    fn enter_dollar(&mut self) {
        if !self.in_dollar {
            if self.buffer.is_empty() {
                // Starting with dollar quote - prepend empty quotes to indicate no prefix
                // GNU coreutils does this for strings that start with only invalid bytes
                self.buffer.extend(b"''$'");
            } else {
                // Had previous content
                if self.in_quote_section {
                    // Close the existing quote section
                    self.buffer.push(b'\'');
                    self.in_quote_section = false;
                } else {
                    // We have unquoted content - need to quote it first
                    let mut temp = Vec::with_capacity(self.buffer.len() + 2);
                    temp.push(b'\'');
                    temp.extend(&self.buffer);
                    temp.push(b'\'');
                    self.buffer = temp;
                }
                self.buffer.extend(b"$'");
            }
            self.in_dollar = true;
        }
    }

    fn exit_dollar(&mut self) {
        if self.in_dollar {
            // Close dollar quote
            // Don't start a new quote section - let finalize handle outer quoting
            self.buffer.push(b'\'');
            self.in_dollar = false;
        }
    }
}

impl Quoter for EscapedShellQuoter<'_> {
    fn push_char(&mut self, input: char) {
        let escaped = EscapedChar::new_shell(input, true, self.quotes);
        match escaped.state {
            // Single quotes need escaping - check BEFORE general Char(x)
            EscapeState::Backslash('\'') | EscapeState::Char('\'') => {
                if self.in_dollar || self.commit_dollar {
                    // In dollar mode OR commit_dollar mode (printf %q), always escape as \'
                    self.must_quote = true;
                    self.buffer.extend(b"\\'");
                } else {
                    // Selective mode (ls), not in dollar
                    self.must_quote = true;
                    if self.quotes == Quotes::Double {
                        // Inside double quotes, single quotes don't need escaping
                        self.buffer.push(b'\'');
                    } else {
                        // Using single quotes: use '\'' escape sequence
                        self.buffer.extend(b"'\\''");
                    }
                }
            }
            EscapeState::Backslash('\\') => {
                if self.in_dollar {
                    // In committed dollar mode, escape as \\
                    self.must_quote = true;
                    self.buffer.extend(b"\\\\");
                } else {
                    self.enter_dollar();
                    self.must_quote = true;
                    self.buffer.extend(b"\\\\");
                }
            }
            EscapeState::Char(x) => {
                if self.in_dollar {
                    if self.commit_dollar {
                        // In committed dollar mode (printf), regular chars are literal
                        self.buffer.extend(x.to_string().as_bytes());
                    } else {
                        // In selective dollar mode (ls), exit dollar and start new quoted section
                        self.exit_dollar();
                        let quote = if self.quotes == Quotes::Single {
                            b'\''
                        } else {
                            b'"'
                        };
                        self.buffer.push(quote);
                        self.in_quote_section = true;
                        self.buffer.extend(x.to_string().as_bytes());
                    }
                } else {
                    // Not in dollar mode - just add the character
                    // Outer quoting will be handled in finalize if needed
                    self.buffer.extend(x.to_string().as_bytes());
                }
            }
            EscapeState::ForceQuote(x) => {
                self.must_quote = true;
                if self.in_dollar {
                    if self.commit_dollar {
                        // Committed dollar mode (printf): metacharacters are literal inside $'...'
                        self.buffer.extend(x.to_string().as_bytes());
                    } else {
                        // Selective dollar mode (ls): exit dollar and start new quoted section
                        self.exit_dollar();
                        self.buffer.push(b'\'');
                        self.in_quote_section = true;
                        self.buffer.extend(x.to_string().as_bytes());
                    }
                } else {
                    // Not in dollar mode
                    if self.commit_dollar {
                        // printf %q: backslash-escape
                        self.buffer.push(b'\\');
                        self.buffer.extend(x.to_string().as_bytes());
                    } else {
                        // ls: will be wrapped in outer quotes, no escaping needed
                        self.buffer.extend(x.to_string().as_bytes());
                    }
                }
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

    fn finalize(mut self: Box<Self>) -> Vec<u8> {
        // Close dollar quote if we ended in committed dollar mode
        if self.in_dollar {
            self.buffer.push(b'\'');
            return self.buffer; // Committed dollar-quoted strings don't need outer quotes
        }

        // Empty string special case
        if self.reference.is_empty() {
            return b"''".to_vec();
        }

        // Close any open quote section we opened after exiting dollar mode
        if self.in_quote_section {
            let quote = if self.quotes == Quotes::Single {
                b'\''
            } else {
                b'"'
            };
            self.buffer.push(quote);
            // Dollar-quoted sections handle their own escaping - no outer quotes needed
            return self.buffer;
        }

        // If buffer contains dollar-quoted sections, we're done
        if self.buffer.windows(2).any(|w| w == b"$'") {
            return self.buffer;
        }

        // For strings without dollar quotes, add outer quotes if needed
        // printf %q (commit_dollar): no outer quotes needed
        // ls (selective): add outer quotes when must_quote OR always_quote OR starts with special chars
        let contains_quote_chars = bytes_start_with(self.reference, SPECIAL_SHELL_CHARS_START);
        if !self.commit_dollar && (self.must_quote || self.always_quote || contains_quote_chars) {
            let mut quoted = Vec::with_capacity(self.buffer.len() + 2);
            let quote = if self.quotes == Quotes::Single {
                b'\''
            } else {
                b'"'
            };
            quoted.push(quote);
            quoted.extend(self.buffer);
            quoted.push(quote);
            quoted
        } else {
            self.buffer
        }
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
