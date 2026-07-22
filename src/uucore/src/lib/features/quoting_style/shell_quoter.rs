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

    /// Whether to always quote the output
    always_quote: bool,

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
        let (quotes, must_quote) =
            initial_quoting_with_show_control(reference, dirname, always_quote, show_control);
        Self {
            reference,
            quotes,
            show_control,
            always_quote,
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
        finalize_shell_quoter(
            self.buffer,
            self.reference,
            self.must_quote || self.always_quote,
            self.quotes,
        )
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
        // true (printf %q): use selective dollar-quoting
        // false (ls): use selective dollar-quoting
        // Both modes use selective quoting: enter $'...' only for control chars
        let commit_dollar = commit_dollar_mode;

        Self {
            reference,
            quotes,
            always_quote,
            commit_dollar,
            must_quote,
            in_dollar: false, // Never start in dollar mode - enter it dynamically
            in_quote_section: false,
            buffer: Vec::with_capacity(size_hint),
        }
    }

    fn enter_dollar(&mut self) {
        if !self.in_dollar {
            if self.in_quote_section {
                // Close any existing quote section first
                self.buffer.push(b'\'');
                self.in_quote_section = false;
            } else if !self.commit_dollar
                && !self.buffer.is_empty()
                && !self.buffer.windows(2).any(|w| w == b"$'")
            {
                // ls mode (not printf %q): Buffer has content but no dollar quotes - wrap it
                let quote = if self.quotes == Quotes::Single {
                    b'\''
                } else {
                    b'"'
                };
                let mut quoted = Vec::with_capacity(self.buffer.len() + 2);
                quoted.push(quote);
                quoted.extend_from_slice(&self.buffer);
                quoted.push(quote);
                self.buffer = quoted;
            } else if !self.commit_dollar && self.buffer.is_empty() {
                // ls mode: When entering dollar mode with empty buffer (entire string needs escaping),
                // prefix with empty quote '' to match GNU behavior
                self.buffer.extend(b"''");
            }
            // If buffer already contains $'...' just append next $'
            self.buffer.extend(b"$'");
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
                if self.in_dollar {
                    // Inside $'...' section - need to exit, then handle apostrophe
                    self.exit_dollar(); // This adds closing '
                    self.must_quote = true;
                    // After exit_dollar's closing ', add the escaped single quote
                    // Result: $'\001'\''$'\001'
                    self.buffer.extend(b"\\'");
                } else if self.commit_dollar {
                    // printf %q mode, not in dollar section
                    self.must_quote = true;
                    // Special case: standalone single quote uses double quotes
                    if self.buffer.is_empty() && self.reference.len() == 1 {
                        self.buffer.extend(b"\"'\"");
                    } else {
                        // Embedded quote - backslash-escape it
                        self.buffer.extend(b"\\'");
                    }
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
            EscapeState::Backslash(x) => {
                // Control character escapes (\n, \t, \r, etc.) or single quote
                // These MUST use $'...' syntax to preserve the escape sequence
                if !self.in_dollar {
                    self.enter_dollar();
                }
                self.must_quote = true;
                self.buffer.push(b'\\');
                self.buffer.extend(x.to_string().as_bytes());
            }
            EscapeState::Char(x) => {
                if self.in_dollar {
                    if self.commit_dollar {
                        // In committed dollar mode (printf), exit and write regular char as-is
                        self.exit_dollar();
                        self.buffer.extend(x.to_string().as_bytes());
                    } else {
                        // In selective dollar mode (ls), exit dollar and start new quoted section
                        self.exit_dollar();
                        self.buffer.push(b'\'');
                        self.in_quote_section = true;
                        self.buffer.extend(x.to_string().as_bytes());
                    }
                } else {
                    // Not in dollar mode - just add the character
                    // Quoting will be handled by enter_dollar (when control chars appear) or finalize
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
                        // printf %q: backslash-escape the special character
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
                // Don't exit - let regular chars exit when needed for selective quoting
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

        // Check if we need outer quotes
        let contains_quote_chars = bytes_start_with(self.reference, SPECIAL_SHELL_CHARS_START);
        let should_quote = self.must_quote || self.always_quote || contains_quote_chars;

        // If buffer contains dollar-quoted sections and doesn't need outer quotes, we're done
        if self.buffer.windows(2).any(|w| w == b"$'") && !should_quote {
            return self.buffer;
        }

        // For printf %q (commit_dollar=true), if the buffer already contains quotes (e.g., "'"
        // for a standalone single quote), don't add outer quotes
        if self.commit_dollar
            && (self.buffer.starts_with(b"\"'\"")
                || self.buffer.starts_with(b"'")
                || self.buffer.starts_with(b"\""))
        {
            return self.buffer;
        }

        // For printf %q (commit_dollar=true), don't add outer quotes
        if self.commit_dollar {
            return self.buffer;
        }

        if should_quote {
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
    initial_quoting_with_show_control(input, dirname, always_quote, true)
}

/// Deduce the initial quoting status, with awareness of whether control chars will be shown
fn initial_quoting_with_show_control(
    input: &[u8],
    dirname: bool,
    always_quote: bool,
    _show_control: bool,
) -> (Quotes, bool) {
    // For NonEscapedShellQuoter, control chars don't trigger quoting.
    // When show_control=false, they become '?' which isn't special.
    // When show_control=true, they're shown as-is but still don't trigger quoting
    // (unlike EscapedShellQuoter which uses dollar-quoting for them).
    // Only characters in shell_escaped_char_set trigger quoting.

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

    if (must_quote || contains_quote_chars) && quotes != Quotes::None {
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
