// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Determine the strategy for breaking up the input (file or stdin) into chunks
//! based on the command line options

use crate::{OPT_BYTES, OPT_LINE_BYTES, OPT_LINES, OPT_NUMBER};
use clap::{ArgMatches, parser::ValueSource};
use thiserror::Error;
use uucore::{
    display::Quotable,
    parser::parse_size::{ParseSizeError, parse_size_u64, parse_size_u64_max},
};

/// Sub-strategy of the [`Strategy::Number`]
/// Splitting a file into a specific number of chunks.
#[derive(Debug, PartialEq)]
pub enum NumberType {
    /// Split into a specific number of chunks by byte.
    Bytes(u64),

    /// Split into a specific number of chunks by byte
    /// but output only the *k*th chunk.
    KthBytes(u64, u64),

    /// Split into a specific number of chunks by line (approximately).
    Lines(u64),

    /// Split into a specific number of chunks by line
    /// (approximately), but output only the *k*th chunk.
    KthLines(u64, u64),

    /// Assign lines via round-robin to the specified number of output chunks.
    RoundRobin(u64),

    /// Assign lines via round-robin to the specified number of output
    /// chunks, but output only the *k*th chunk.
    KthRoundRobin(u64, u64),
}

impl NumberType {
    /// The number of chunks for this number type.
    pub fn num_chunks(&self) -> u64 {
        match self {
            Self::Bytes(n) => *n,
            Self::KthBytes(_, n) => *n,
            Self::Lines(n) => *n,
            Self::KthLines(_, n) => *n,
            Self::RoundRobin(n) => *n,
            Self::KthRoundRobin(_, n) => *n,
        }
    }
}

/// An error due to an invalid parameter to the `-n` command-line option.
#[derive(Debug, PartialEq, Error)]
pub enum NumberTypeError {
    /// The number of chunks was invalid.
    ///
    /// This can happen if the value of `N` in any of the following
    /// command-line options is not a positive integer:
    ///
    /// ```ignore
    /// -n N
    /// -n K/N
    /// -n l/N
    /// -n l/K/N
    /// -n r/N
    /// -n r/K/N
    /// ```
    #[error("invalid number of chunks: {}", .0.quote())]
    NumberOfChunks(String),

    /// The chunk number was invalid.
    ///
    /// This can happen if the value of `K` in any of the following
    /// command-line options is not a positive integer
    /// or if `K` is 0
    /// or if `K` is greater than `N`:
    ///
    /// ```ignore
    /// -n K/N
    /// -n l/K/N
    /// -n r/K/N
    /// ```
    #[error("invalid chunk number: {}", .0.quote())]
    ChunkNumber(String),
}

impl NumberType {
    /// Parse a `NumberType` from a string.
    ///
    /// The following strings are valid arguments:
    ///
    /// ```ignore
    /// "N"
    /// "K/N"
    /// "l/N"
    /// "l/K/N"
    /// "r/N"
    /// "r/K/N"
    /// ```
    ///
    /// The `N` represents the number of chunks and the `K` represents
    /// a chunk number.
    ///
    /// # Errors
    ///
    /// If the string is not one of the valid number types,
    /// if `K` is not a non-negative integer,
    /// or if `K` is 0,
    /// or if `N` is not a positive integer,
    /// or if `K` is greater than `N`
    /// then this function returns [`NumberTypeError`].
    fn from(s: &str) -> Result<Self, NumberTypeError> {
        fn is_invalid_chunk(chunk_number: u64, num_chunks: u64) -> bool {
            chunk_number > num_chunks || chunk_number == 0
        }
        let mut parts = s.splitn(4, '/');
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(n_str), None, None, None) => {
                let num_chunks = parse_size_u64(n_str)
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                if num_chunks > 0 {
                    Ok(Self::Bytes(num_chunks))
                } else {
                    Err(NumberTypeError::NumberOfChunks(s.to_string()))
                }
            }
            (Some(k_str), Some(n_str), None, None)
                if !k_str.starts_with('l') && !k_str.starts_with('r') =>
            {
                let num_chunks = parse_size_u64(n_str)
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                let chunk_number = parse_size_u64(k_str)
                    .map_err(|_| NumberTypeError::ChunkNumber(k_str.to_string()))?;
                if is_invalid_chunk(chunk_number, num_chunks) {
                    return Err(NumberTypeError::ChunkNumber(k_str.to_string()));
                }
                Ok(Self::KthBytes(chunk_number, num_chunks))
            }
            (Some("l"), Some(n_str), None, None) => {
                let num_chunks = parse_size_u64(n_str)
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                Ok(Self::Lines(num_chunks))
            }
            (Some("l"), Some(k_str), Some(n_str), None) => {
                let num_chunks = parse_size_u64(n_str)
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                let chunk_number = parse_size_u64(k_str)
                    .map_err(|_| NumberTypeError::ChunkNumber(k_str.to_string()))?;
                if is_invalid_chunk(chunk_number, num_chunks) {
                    return Err(NumberTypeError::ChunkNumber(k_str.to_string()));
                }
                Ok(Self::KthLines(chunk_number, num_chunks))
            }
            (Some("r"), Some(n_str), None, None) => {
                let num_chunks = parse_size_u64(n_str)
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                Ok(Self::RoundRobin(num_chunks))
            }
            (Some("r"), Some(k_str), Some(n_str), None) => {
                let num_chunks = parse_size_u64(n_str)
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                let chunk_number = parse_size_u64(k_str)
                    .map_err(|_| NumberTypeError::ChunkNumber(k_str.to_string()))?;
                if is_invalid_chunk(chunk_number, num_chunks) {
                    return Err(NumberTypeError::ChunkNumber(k_str.to_string()));
                }
                Ok(Self::KthRoundRobin(chunk_number, num_chunks))
            }
            _ => Err(NumberTypeError::NumberOfChunks(s.to_string())),
        }
    }
}

/// The strategy for breaking up the input file into chunks.
pub enum Strategy {
    /// Each chunk has the specified number of lines.
    Lines(u64),

    /// Each chunk has the specified number of bytes.
    Bytes(u64),

    /// Each chunk has as many lines as possible without exceeding the
    /// specified number of bytes.
    LineBytes(u64),

    /// Split the file into this many chunks.
    ///
    /// There are several sub-strategies available, as defined by
    /// [`NumberType`].
    Number(NumberType),
}

/// An error when parsing a chunking strategy from command-line arguments.
#[derive(Debug, Error)]
pub enum StrategyError {
    /// Invalid number of lines.
    #[error("invalid number of lines: {0}")]
    Lines(ParseSizeError),

    /// Invalid number of bytes.
    #[error("invalid number of bytes: {0}")]
    Bytes(ParseSizeError),

    /// Invalid number type.
    #[error("{0}")]
    NumberType(NumberTypeError),

    /// Multiple chunking strategies were specified (but only one should be).
    #[error("cannot split in more than one way")]
    MultipleWays,
}

impl Strategy {
    /// Parse a strategy from the command-line arguments.
    pub fn from(matches: &ArgMatches, obs_lines: Option<&str>) -> Result<Self, StrategyError> {
        fn get_and_parse(
            matches: &ArgMatches,
            option: &str,
            strategy: fn(u64) -> Strategy,
            error: fn(ParseSizeError) -> StrategyError,
        ) -> Result<Strategy, StrategyError> {
            let s = matches.get_one::<String>(option).unwrap();
            let n = parse_size_u64_max(s).map_err(error)?;
            if n > 0 {
                Ok(strategy(n))
            } else {
                Err(error(ParseSizeError::ParseFailure(s.to_string())))
            }
        }
        // Check that the user is not specifying more than one strategy.
        //
        // Note: right now, this exact behavior cannot be handled by
        // overrides_with_all() due to obsolete lines value option
        match (
            obs_lines,
            matches.value_source(OPT_LINES) == Some(ValueSource::CommandLine),
            matches.value_source(OPT_BYTES) == Some(ValueSource::CommandLine),
            matches.value_source(OPT_LINE_BYTES) == Some(ValueSource::CommandLine),
            matches.value_source(OPT_NUMBER) == Some(ValueSource::CommandLine),
        ) {
            (Some(v), false, false, false, false) => {
                let v = parse_size_u64_max(v).map_err(|_| {
                    StrategyError::Lines(ParseSizeError::ParseFailure(v.to_string()))
                })?;
                if v > 0 {
                    Ok(Self::Lines(v))
                } else {
                    Err(StrategyError::Lines(ParseSizeError::ParseFailure(
                        v.to_string(),
                    )))
                }
            }
            (None, false, false, false, false) => Ok(Self::Lines(1000)),
            (None, true, false, false, false) => {
                get_and_parse(matches, OPT_LINES, Self::Lines, StrategyError::Lines)
            }
            (None, false, true, false, false) => {
                get_and_parse(matches, OPT_BYTES, Self::Bytes, StrategyError::Bytes)
            }
            (None, false, false, true, false) => get_and_parse(
                matches,
                OPT_LINE_BYTES,
                Self::LineBytes,
                StrategyError::Bytes,
            ),
            (None, false, false, false, true) => {
                let s = matches.get_one::<String>(OPT_NUMBER).unwrap();
                let number_type = NumberType::from(s).map_err(StrategyError::NumberType)?;
                Ok(Self::Number(number_type))
            }
            _ => Err(StrategyError::MultipleWays),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{strategy::NumberType, strategy::NumberTypeError};

    #[test]
    fn test_number_type_from() {
        assert_eq!(NumberType::from("123").unwrap(), NumberType::Bytes(123));
        assert_eq!(NumberType::from("l/123").unwrap(), NumberType::Lines(123));
        assert_eq!(
            NumberType::from("l/123/456").unwrap(),
            NumberType::KthLines(123, 456)
        );
        assert_eq!(
            NumberType::from("r/123").unwrap(),
            NumberType::RoundRobin(123)
        );
        assert_eq!(
            NumberType::from("r/123/456").unwrap(),
            NumberType::KthRoundRobin(123, 456)
        );
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_number_type_from_error() {
        assert_eq!(
            NumberType::from("xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("l/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("l/123/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("l/abc/456").unwrap_err(),
            NumberTypeError::ChunkNumber("abc".to_string())
        );
        assert_eq!(
            NumberType::from("l/456/123").unwrap_err(),
            NumberTypeError::ChunkNumber("456".to_string())
        );
        assert_eq!(
            NumberType::from("r/456/123").unwrap_err(),
            NumberTypeError::ChunkNumber("456".to_string())
        );
        assert_eq!(
            NumberType::from("456/123").unwrap_err(),
            NumberTypeError::ChunkNumber("456".to_string())
        );
        // In GNU split, the number of chunks get precedence:
        //
        //     $ split -n l/abc/xyz
        //     split: invalid number of chunks: ‘xyz’
        //
        assert_eq!(
            NumberType::from("l/abc/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("r/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("r/123/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("r/abc/456").unwrap_err(),
            NumberTypeError::ChunkNumber("abc".to_string())
        );
        // In GNU split, the number of chunks get precedence:
        //
        //     $ split -n r/abc/xyz
        //     split: invalid number of chunks: ‘xyz’
        //
        assert_eq!(
            NumberType::from("r/abc/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
    }

    #[test]
    fn test_number_type_num_chunks() {
        assert_eq!(NumberType::from("123").unwrap().num_chunks(), 123);
        assert_eq!(NumberType::from("123/456").unwrap().num_chunks(), 456);
        assert_eq!(NumberType::from("l/123").unwrap().num_chunks(), 123);
        assert_eq!(NumberType::from("l/123/456").unwrap().num_chunks(), 456);
        assert_eq!(NumberType::from("r/123").unwrap().num_chunks(), 123);
        assert_eq!(NumberType::from("r/123/456").unwrap().num_chunks(), 456);
    }
}
