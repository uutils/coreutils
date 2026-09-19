// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! No-op stand-in for the `diagnostics` module, compiled when the
//! `diagnostics` cargo feature is off.
//!
//! It mirrors the real API so callers need no `cfg` of their own:
//! [`enabled`] is always `false`, the locators find nothing and the render
//! methods render nothing, so every caller falls back to the plain one-line
//! message it would print anywhere but a terminal.

use std::ffi::{OsStr, OsString};
use std::ops::Range;

/// Always `false`: rendering is compiled out.
pub fn enabled() -> bool {
    false
}

/// Always `None`: rendering is compiled out.
pub fn capture(_args: &[OsString]) -> Option<Vec<OsString>> {
    None
}

/// Always `None`: rendering is compiled out.
pub fn operands(_args: &[OsString]) -> Option<Vec<OsString>> {
    None
}

// Boundary arithmetic is real even when rendering is not: a caller may floor
// an offset before it knows whether anything will be drawn. It is the one part
// of this module that is not a no-op, so it is shared with the real one rather
// than restated here.
pub use crate::features::diagnostics_boundary::{
    OptionValue, ValueOptions, char_span, floor_boundary, list_items,
};

/// Always the error itself: nothing is ever drawn to replace it.
pub fn error_after_report<E: Into<Box<dyn crate::error::UError>>>(
    _diag_args: Option<&[OsString]>,
    error: E,
    _draw: impl FnOnce(&[OsString], &E) -> bool,
) -> Box<dyn crate::error::UError> {
    error.into()
}

/// A snapshot of nothing: it finds nothing and renders nothing.
///
/// Deliberately derives nothing the real `Snapshot` does not, so that what a
/// caller may do with one does not depend on whether the feature is on.
pub struct Snapshot;

#[allow(clippy::unused_self)]
impl Snapshot {
    pub fn new<S: AsRef<OsStr>>(_args: &[S]) -> Self {
        Self
    }

    pub fn with_program<S: AsRef<OsStr>>(_args: &[S]) -> Self {
        Self
    }

    pub fn from_bytes<S: AsRef<[u8]>>(_args: &[S]) -> Self {
        Self
    }

    pub fn is_empty(&self) -> bool {
        true
    }

    pub fn index_of(&self, _arg: &OsStr) -> Option<usize> {
        None
    }

    pub fn index_of_value(
        &self,
        _operand: &str,
        _short: Option<char>,
        _long: Option<&str>,
    ) -> Option<usize> {
        None
    }

    pub fn index_of_positional(&self, _n: usize) -> Option<usize> {
        None
    }

    pub fn index_of_operand(&self, _n: usize, _options: &ValueOptions) -> Option<usize> {
        None
    }

    pub fn render(
        &self,
        _index: usize,
        _message: &str,
        _label: Option<&str>,
        _help: Option<&str>,
    ) -> bool {
        false
    }

    pub fn render_inside_at(
        &self,
        _index: usize,
        _operand: &str,
        _range: Range<usize>,
        _message: &str,
        _label: Option<&str>,
        _help: Option<&str>,
    ) -> bool {
        false
    }

    pub fn render_option(
        &self,
        _option: &OptionValue,
        _range: Range<usize>,
        _message: &str,
        _label: Option<&str>,
        _help: Option<&str>,
    ) -> bool {
        false
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_option_value(
        &self,
        _operand: &str,
        _short: Option<char>,
        _long: Option<&str>,
        _range: Range<usize>,
        _message: &str,
        _label: Option<&str>,
        _help: Option<&str>,
    ) -> bool {
        false
    }
}
