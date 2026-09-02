// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions to parse modes

// spell-checker:ignore (vars) fperm srwx

use std::fmt::{self, Display};
use std::num::ParseIntError;
use std::ops::Range;

#[cfg(windows)]
use libc::umask;

use crate::translate;

/// A mode string that does not parse, and the part of it that is at fault.
///
/// `span` is a byte range inside the mode string that was handed to the parser,
/// so that a caller can point a caret at the one clause — often the one
/// character — that broke it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeError {
    detail: Detail,
    pub span: Range<usize>,
    pub kind: ModeErrorKind,
}

/// What went wrong, for callers that want to say more than the message does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeErrorKind {
    /// Something other than `+`, `-` or `=` where an operator was expected.
    InvalidOperator,
    /// Who the clause applies to, and then nothing.
    MissingOperator,
    /// A numeric mode that is not octal, or is out of range.
    InvalidNumber,
}

/// What a message would need to say, kept unformatted until something asks for
/// it.
///
/// Formatting here means [`translate!`], and a translation needs a localizer.
/// Building the message when the error is constructed would put that setup on
/// the *parsing* path — which is the success path — for a string that is
/// thrown away whenever the mode is valid. Worse, a caller that has not
/// installed a localizer yet cannot install one afterwards to fix it: the
/// message is already frozen as the message id. Holding the pieces instead
/// means only [`Display`] pays, and only when someone prints.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Detail {
    /// A clause that ran out where an operator was expected.
    UnexpectedEnd,
    /// Something other than `+`, `-` or `=`, and what stood there instead.
    InvalidOperator(char),
    /// A clause naming who to change but never what, and the clause itself.
    MissingOperator(String),
    /// A numeric mode that is not octal, and why the digits did not parse.
    NotOctal(ParseIntError),
    /// A numeric mode above `7777`, and the value that was read.
    TooLarge(u32),
}

impl Detail {
    /// The coarse classification that goes with these particulars.
    ///
    /// Deriving it here rather than passing it in is what keeps [`ModeError`]'s
    /// `kind` from drifting away from what the message actually says.
    fn kind(&self) -> ModeErrorKind {
        match self {
            Self::UnexpectedEnd | Self::MissingOperator(_) => ModeErrorKind::MissingOperator,
            Self::InvalidOperator(_) => ModeErrorKind::InvalidOperator,
            Self::NotOctal(_) | Self::TooLarge(_) => ModeErrorKind::InvalidNumber,
        }
    }
}

impl ModeError {
    fn new(detail: Detail, span: Range<usize>) -> Self {
        Self {
            kind: detail.kind(),
            detail,
            span,
        }
    }

    /// Move the range so that it is relative to a string starting `offset`
    /// bytes earlier.
    fn shift(self, offset: usize) -> Self {
        Self {
            span: self.span.start + offset..self.span.end + offset,
            ..self
        }
    }

    /// Render this error against `args`, where the mode is the value of the
    /// `-m`/`--mode` option.
    ///
    /// # Arguments
    ///
    /// * `args` - The argument list the mode came from, without the program
    ///   name — as [`crate::diagnostics::operands`] returns it, since the other
    ///   renderer here takes a positional index into the same list.
    /// * `mode` - The whole mode operand.
    /// * `clause_start` - Where the clause that failed begins inside `mode`,
    ///   since a mode is parsed one comma-separated clause at a time.
    /// * `message` - The headline, already localized.
    ///
    /// # Returns
    ///
    /// `false` when the mode cannot be found among the arguments, in which case
    /// the caller should fall back to the plain one-line message.
    pub fn render_mode_value(
        &self,
        args: &[std::ffi::OsString],
        mode: &str,
        clause_start: usize,
        message: &str,
    ) -> bool {
        let (label, help) = self.describe();
        crate::diagnostics::Snapshot::new(args).render_option_value(
            mode,
            Some('m'),
            Some("mode"),
            self.clause_span(clause_start),
            message,
            label.as_deref(),
            help.as_deref(),
        )
    }

    /// Render this error against `args`, with the argument carrying the mode
    /// already located by the caller.
    ///
    /// # Arguments
    ///
    /// * `args` - The argument list the mode came from, without the program
    ///   name.
    /// * `index` - Position of the argument carrying the mode inside `args`.
    /// * `mode` - The mode as it appears in that argument.
    /// * `clause_start` - Where the clause that failed begins inside `mode`,
    ///   since a mode is parsed one comma-separated clause at a time.
    /// * `message` - The headline, already localized.
    ///
    /// # Returns
    ///
    /// `false` when nothing could be rendered, in which case the caller should
    /// fall back to the plain one-line message.
    pub fn render_at(
        &self,
        args: &[std::ffi::OsString],
        index: usize,
        mode: &str,
        clause_start: usize,
        message: &str,
    ) -> bool {
        let (label, help) = self.describe();
        crate::diagnostics::Snapshot::new(args).render_inside_at(
            index,
            mode,
            self.clause_span(clause_start),
            message,
            label.as_deref(),
            help.as_deref(),
        )
    }

    /// The caret label for this error, translated, and the advice that goes
    /// under it.
    ///
    /// Labelled only where a label would add to the message, per the
    /// convention in [`crate::diagnostics`].
    fn describe(&self) -> (Option<String>, Option<String>) {
        let label = match self.kind {
            // The message already names the expected operators.
            ModeErrorKind::InvalidOperator => None,
            ModeErrorKind::MissingOperator => Some("mode-diag-label-missing-operator"),
            ModeErrorKind::InvalidNumber => Some("mode-diag-label-invalid-number"),
        };
        (
            label.map(|label| translate!(label)),
            Some(translate!("mode-diag-help-syntax")),
        )
    }

    /// Where this error sits inside the whole mode, given where the clause it
    /// was raised in begins.
    fn clause_span(&self, clause_start: usize) -> Range<usize> {
        clause_start + self.span.start..clause_start + self.span.end
    }
}

impl Display for ModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The one place the message is built, so that a parse that succeeds --
        // or an error that is only ever inspected through `kind` -- never
        // touches the localizer.
        match &self.detail {
            Detail::UnexpectedEnd => f.write_str(&translate!("mode-error-unexpected-end")),
            Detail::InvalidOperator(operator) => f.write_str(&translate!(
                "mode-error-invalid-operator",
                "operator" => *operator
            )),
            Detail::MissingOperator(clause) => f.write_str(&translate!(
                "mode-error-missing-operator",
                "mode" => clause.clone()
            )),
            // The text comes from `std`, which has no catalogue of ours to
            // draw on, so it is passed through rather than given an id that
            // could never be translated.
            Detail::NotOctal(error) => write!(f, "{error}"),
            Detail::TooLarge(mode) => f.write_str(&translate!(
                "mode-error-too-large",
                "mode" => format!("{mode:o}")
            )),
        }
    }
}

impl std::error::Error for ModeError {}

pub fn parse_numeric(fperm: u32, mode: &str, considering_dir: bool) -> Result<u32, ModeError> {
    let original = mode;
    let (op, pos) = parse_op(mode).map_or_else(|_| (None, 0), |(op, pos)| (Some(op), pos));
    let digits = mode[pos..].trim();
    let mode = digits;
    let change = if mode.is_empty() {
        0
    } else {
        let at = original.len() - original.trim_end().len();
        u32::from_str_radix(mode, 8)
            .map_err(|e| ModeError::new(Detail::NotOctal(e), pos..original.len() - at))?
    };
    if change > 0o7777 {
        Err(ModeError::new(Detail::TooLarge(change), 0..original.len()))
    } else {
        Ok(match op {
            Some('+') => fperm | change,
            Some('-') => fperm & !change,
            // If this is a directory, we keep the setgid and setuid bits,
            // unless the mode contains 5 or more octal digits or the mode is "="
            None if considering_dir && mode.len() < 5 => change | (fperm & (0o4000 | 0o2000)),
            None | Some('=') => change,
            Some(_) => unreachable!(),
        })
    }
}

pub fn parse_symbolic(
    mut fperm: u32,
    mode: &str,
    umask: u32,
    considering_dir: bool,
) -> Result<u32, ModeError> {
    let original = mode;
    // Everything below is a suffix of `original`, so how much is left is also
    // where we are.
    let at = |rest: &str| original.len() - rest.len();

    let (mask, pos) = parse_levels(mode);
    if pos == mode.len() {
        // Who the clause applies to, and then nothing: no operator followed.
        return Err(ModeError::new(
            Detail::MissingOperator(mode.to_owned()),
            0..mode.len(),
        ));
    }
    let respect_umask = pos == 0;
    let mut mode = &mode[pos..];
    while !mode.is_empty() {
        let (op, pos) = parse_op(mode).map_err(|err| err.shift(at(mode)))?;
        mode = &mode[pos..];
        let (mut srwx, pos) = parse_change(mode, fperm, considering_dir);
        if respect_umask {
            srwx &= !umask;
        }
        mode = &mode[pos..];
        match op {
            '+' => fperm |= srwx & mask,
            '-' => fperm &= !(srwx & mask),
            '=' => {
                if considering_dir {
                    // keep the setgid and setuid bits for directories
                    srwx |= fperm & (0o4000 | 0o2000);
                }
                fperm = (fperm & !mask) | (srwx & mask);
            }
            _ => unreachable!(),
        }
    }
    Ok(fperm)
}

fn parse_levels(mode: &str) -> (u32, usize) {
    let mut mask = 0;
    let mut pos = 0;
    for ch in mode.chars() {
        mask |= match ch {
            'u' => 0o4700,
            'g' => 0o2070,
            'o' => 0o1007,
            'a' => 0o7777,
            _ => break,
        };
        pos += 1;
    }
    if pos == 0 {
        mask = 0o7777; // default to 'a'
    }
    (mask, pos)
}

fn parse_op(mode: &str) -> Result<(char, usize), ModeError> {
    let Some(ch) = mode.chars().next() else {
        return Err(ModeError::new(Detail::UnexpectedEnd, 0..0));
    };
    match ch {
        '+' | '-' | '=' => Ok((ch, 1)),
        _ => Err(ModeError::new(
            Detail::InvalidOperator(ch),
            0..ch.len_utf8(),
        )),
    }
}

fn parse_change(mode: &str, fperm: u32, considering_dir: bool) -> (u32, usize) {
    let mut srwx = 0;
    let mut pos = 0;
    for ch in mode.chars() {
        match ch {
            'r' => srwx |= 0o444,
            'w' => srwx |= 0o222,
            'x' => srwx |= 0o111,
            'X' => {
                if considering_dir || (fperm & 0o0111) != 0 {
                    srwx |= 0o111;
                }
            }
            's' => srwx |= 0o4000 | 0o2000,
            't' => srwx |= 0o1000,
            'u' => srwx = (fperm & 0o700) | ((fperm >> 3) & 0o070) | ((fperm >> 6) & 0o007),
            'g' => srwx = ((fperm << 3) & 0o700) | (fperm & 0o070) | ((fperm >> 3) & 0o007),
            'o' => srwx = ((fperm << 6) & 0o700) | ((fperm << 3) & 0o070) | (fperm & 0o007),
            _ => break,
        }
        if ch == 'u' || ch == 'g' || ch == 'o' {
            // symbolic modes only allows perms to be a single letter of 'ugo'
            // therefore this must either be the first char or it is unexpected
            if pos != 0 {
                break;
            }
            pos = 1;
            break;
        }
        pos += 1;
    }
    if pos == 0 {
        srwx = 0;
    }
    (srwx, pos)
}

/// Modify a file mode based on a user-supplied string.
/// Supports comma-separated mode strings like "ug+rwX,o+rX" (same as chmod).
pub fn parse_chmod(
    current_mode: u32,
    mode_string: &str,
    considering_dir: bool,
    umask: u32,
) -> Result<u32, ModeError> {
    let mut new_mode: u32 = current_mode;

    // Split by commas and process each mode part sequentially
    let mut offset = 0;
    for raw_part in mode_string.split(',') {
        let start = offset + (raw_part.len() - raw_part.trim_start().len());
        // Past the part and the comma that followed it.
        offset += raw_part.len() + 1;

        let mode_part = raw_part.trim();
        if mode_part.is_empty() {
            continue;
        }

        new_mode = if mode_part.chars().any(|c| c.is_ascii_digit()) {
            parse_numeric(new_mode, mode_part, considering_dir)
        } else {
            parse_symbolic(new_mode, mode_part, umask, considering_dir)
        }
        .map_err(|err| err.shift(start))?;
    }

    Ok(new_mode)
}

/// Takes a user-supplied string and tries to parse to u32 mode bitmask.
pub fn parse(mode_string: &str, considering_dir: bool, umask: u32) -> Result<u32, ModeError> {
    parse_chmod(0, mode_string, considering_dir, umask)
}

pub fn get_umask() -> u32 {
    // There's no portable way to read the umask without changing it.
    // We have to replace it and then quickly set it back, hopefully before
    // some other thread is affected.
    // On modern Linux kernels the current umask could instead be read
    // from /proc/self/status. But that's a lot of work.
    #[cfg(unix)]
    {
        use rustix::fs::Mode;
        use rustix::process::umask;

        let mask = umask(Mode::empty());
        let _ = umask(mask);
        mask.bits() as u32
    }

    #[cfg(windows)]
    {
        // SAFETY: umask always succeeds and doesn't operate on memory. Races are
        // possible but it can't violate Rust's guarantees.
        let mask = unsafe { umask(0) };
        unsafe { umask(mask) };
        mask as u32
    }

    // WASI has no umask; return a typical default (022).
    #[cfg(not(any(unix, windows)))]
    {
        0o022
    }
}

#[cfg(test)]
mod tests {

    use super::parse;
    use super::parse_chmod;
    use super::{ModeErrorKind, parse_numeric, parse_symbolic};

    #[test]
    fn test_chmod_symbolic_modes() {
        assert_eq!(parse_chmod(0o666, "u+x", false, 0).unwrap(), 0o766);
        assert_eq!(parse_chmod(0o666, "+x", false, 0).unwrap(), 0o777);
        assert_eq!(parse_chmod(0o666, "a-w", false, 0).unwrap(), 0o444);
        assert_eq!(parse_chmod(0o666, "g-r", false, 0).unwrap(), 0o626);
    }

    #[test]
    fn test_chmod_numeric_modes() {
        assert_eq!(parse_chmod(0o666, "644", false, 0).unwrap(), 0o644);
        assert_eq!(parse_chmod(0o666, "+100", false, 0).unwrap(), 0o766);
        assert_eq!(parse_chmod(0o666, "-4", false, 0).unwrap(), 0o662);
    }

    #[test]
    fn test_parse_numeric_mode() {
        // Simple numeric mode
        assert_eq!(parse("644", false, 0).unwrap(), 0o644);
        assert_eq!(parse("755", false, 0).unwrap(), 0o755);
        assert_eq!(parse("777", false, 0).unwrap(), 0o777);
        assert_eq!(parse("600", false, 0).unwrap(), 0o600);
    }

    #[test]
    fn test_parse_numeric_mode_with_operator() {
        // Numeric mode with + operator
        assert_eq!(parse("+100", false, 0).unwrap(), 0o100);
        assert_eq!(parse("+644", false, 0).unwrap(), 0o644);

        // Numeric mode with - operator (starting from 0, so nothing to remove)
        assert_eq!(parse("-4", false, 0).unwrap(), 0);
        // But if we first set a mode, then remove bits
        assert_eq!(parse("644,-4", false, 0).unwrap(), 0o640);
    }

    #[test]
    fn test_parse_symbolic_mode() {
        // Simple symbolic modes
        assert_eq!(parse("u+x", false, 0).unwrap(), 0o100);
        assert_eq!(parse("g+w", false, 0).unwrap(), 0o020);
        assert_eq!(parse("o+r", false, 0).unwrap(), 0o004);
        assert_eq!(parse("a+x", false, 0).unwrap(), 0o111);
    }

    #[test]
    fn test_parse_symbolic_mode_multiple_permissions() {
        // Multiple permissions in one mode
        assert_eq!(parse("u+rw", false, 0).unwrap(), 0o600);
        assert_eq!(parse("ug+rwx", false, 0).unwrap(), 0o770);
        assert_eq!(parse("a+rwx", false, 0).unwrap(), 0o777);
    }

    #[test]
    fn test_parse_comma_separated_modes() {
        // Comma-separated mode strings (as mentioned in the doc comment)
        assert_eq!(parse("ug+rwX,o+rX", false, 0).unwrap(), 0o664);
        assert_eq!(parse("u+rwx,g+rx,o+r", false, 0).unwrap(), 0o754);
        assert_eq!(parse("u+w,g+w,o+w", false, 0).unwrap(), 0o222);
    }

    #[test]
    fn test_parse_comma_separated_with_spaces() {
        // Comma-separated with spaces (should be trimmed)
        assert_eq!(parse("u+rw, g+rw, o+r", false, 0).unwrap(), 0o664);
        assert_eq!(parse(" u+x , g+x ", false, 0).unwrap(), 0o110);
    }

    #[test]
    fn test_parse_mixed_numeric_and_symbolic() {
        // Mix of numeric and symbolic modes
        assert_eq!(parse("644,u+x", false, 0).unwrap(), 0o744);
        assert_eq!(parse("u+rw,755", false, 0).unwrap(), 0o755);
    }

    #[test]
    fn test_parse_empty_string() {
        // Empty string should return 0
        assert_eq!(parse("", false, 0).unwrap(), 0);
        assert_eq!(parse("   ", false, 0).unwrap(), 0);
        assert_eq!(parse(",,", false, 0).unwrap(), 0);
    }

    #[test]
    fn test_parse_with_umask() {
        // Test with umask (affects symbolic modes when no level is specified)
        let umask = 0o022;
        assert_eq!(parse("+w", false, umask).unwrap(), 0o200);
        // The umask should be respected for symbolic modes without explicit level
    }

    #[test]
    fn test_parse_considering_dir() {
        // Test directory vs file mode differences
        // For directories, X (capital X) should add execute permission
        assert_eq!(parse("a+X", true, 0).unwrap(), 0o111);
        // For files without execute, X should not add execute
        assert_eq!(parse("a+X", false, 0).unwrap(), 0o000);

        // Numeric modes for directories preserve setuid/setgid bits
        assert_eq!(parse("755", true, 0).unwrap(), 0o755);
    }

    #[test]
    fn test_parse_invalid_modes() {
        // Invalid numeric mode (too large)
        assert!(parse("10000", false, 0).is_err());

        // Invalid operator
        assert!(parse("u*rw", false, 0).is_err());

        // Invalid symbolic mode
        assert!(parse("invalid", false, 0).is_err());
    }

    #[test]
    fn test_parse_complex_combinations() {
        // Complex real-world examples
        assert_eq!(parse("u=rwx,g=rx,o=r", false, 0).unwrap(), 0o754);
        // To test removal, we need to first set permissions, then remove them
        assert_eq!(parse("644,a-w", false, 0).unwrap(), 0o444);
        assert_eq!(parse("644,g-r", false, 0).unwrap(), 0o604);
    }

    #[test]
    fn test_parse_sequential_application() {
        // Test that comma-separated modes are applied sequentially
        // First set to 644, then add execute for user
        assert_eq!(parse("644,u+x", false, 0).unwrap(), 0o744);

        // First add user write, then set to 755 (should override)
        assert_eq!(parse("u+w,755", false, 0).unwrap(), 0o755);
    }

    #[test]
    fn a_localizer_installed_after_the_parse_still_formats_the_message() {
        // The point of holding the pieces instead of the finished string: a
        // caller that only sets up localization once something has actually
        // gone wrong -- off the success path -- must still get real text. Build
        // the errors first, install the localizer second, print third.
        //
        // The localizer is thread-local, so this runs somewhere that has none
        // yet rather than wherever the test harness put us.
        std::thread::spawn(|| {
            let errors = [
                parse_symbolic(0, "u*rw", 0, false).unwrap_err(),
                parse_numeric(0, "77777", false).unwrap_err(),
                parse_symbolic(0, "u", 0, false).unwrap_err(),
            ];

            // Before: nothing is localized yet, so every message is its own id.
            for error in &errors {
                assert!(
                    error.to_string().starts_with("mode-error-"),
                    "expected an untranslated id, got {error}"
                );
            }

            let _ = crate::locale::setup_localization("chmod");

            // After: the same errors, now with something to translate against.
            // What language that lands in depends on the environment, so this
            // asserts only what is universal -- that a message came out, and
            // not the id that a message built too early would be stuck with.
            for error in &errors {
                let message = error.to_string();
                assert!(
                    !message.starts_with("mode-error-"),
                    "message was frozen before the localizer existed: {message}"
                );
                assert!(!message.is_empty());
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn the_kind_never_disagrees_with_the_message() {
        // `kind` is derived from the same particulars the message is built
        // from, so the two cannot drift apart.
        assert_eq!(
            parse_symbolic(0, "u*rw", 0, false).unwrap_err().kind,
            ModeErrorKind::InvalidOperator
        );
        assert_eq!(
            parse_symbolic(0, "u", 0, false).unwrap_err().kind,
            ModeErrorKind::MissingOperator
        );
        // A trailing comma opens a clause whose first character is not an
        // operator, rather than one that ends early.
        assert_eq!(
            parse_symbolic(0, "u+rw,", 0, false).unwrap_err().kind,
            ModeErrorKind::InvalidOperator
        );
        assert_eq!(
            parse_numeric(0, "8", false).unwrap_err().kind,
            ModeErrorKind::InvalidNumber
        );
        assert_eq!(
            parse_numeric(0, "77777", false).unwrap_err().kind,
            ModeErrorKind::InvalidNumber
        );
    }
}
