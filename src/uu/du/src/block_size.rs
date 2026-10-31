// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uucore::parser::parse_size::{ParseSizeError, parse_size_non_zero_u64};

#[derive(Clone)]
pub(super) struct BlockSize {
    pub(super) bytes: u64,
    suffix: String,
}

impl BlockSize {
    pub(super) fn new(bytes: u64) -> Self {
        Self {
            bytes,
            suffix: String::new(),
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self, ParseSizeError> {
        let size = value;
        let bytes = parse_size_non_zero_u64(size)?;
        // An omitted multiplier requests a unit label; an explicit multiplier
        // requests just the count, even when that multiplier is one.
        let mut suffix = if size.starts_with(|c: char| c.is_ascii_digit()) {
            String::new()
        } else {
            size.to_owned()
        };
        if let Some(first) = suffix.get_mut(..1) {
            first.make_ascii_uppercase();
        }
        if suffix == "KB" {
            "kB".clone_into(&mut suffix);
        } else if suffix.ends_with('D') {
            suffix.pop();
        }
        Ok(Self { bytes, suffix })
    }

    pub(super) fn format(&self, size: u64) -> String {
        let count = size.div_ceil(self.bytes);
        let mut number = count.to_string();
        number.push_str(&self.suffix);
        number
    }
}
