// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io;

use uucore::{display::Quotable as _, translate};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Error while reading input.
    #[error(transparent)]
    Read(#[from] ReadError),

    /// Error while writing output.
    #[error("{message}: {0}", message = translate!("common-write-error"))]
    Write(io::Error),

    /// The graph contains a cycle.
    #[error("{input}: {message}", input = .0, message = translate!("tsort-error-loop"))]
    Loop(String),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReadError {
    /// The input file is actually a directory.
    #[error("{input}: {read_error}: {message}", input = .0.maybe_quote(), read_error = translate!("common-read-error"), message = translate!("error-is-a-directory-text"))]
    IsDir(String),

    /// The number of tokens in the input data is odd.
    ///
    /// The length of the list of edges must be even because each edge has two
    /// components: a source node and a target node.
    #[error("{input}: {message}", input = .0.maybe_quote(), message = translate!("tsort-error-odd"))]
    NumTokensOdd(String),

    /// Wrapper for bubbling up input IO errors.
    #[error("{message}: {0}", message = translate!("common-read-error"))]
    Io(io::Error),
}
