// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

mod files;
mod watch;

pub use files::NonSeekableReader;
pub use watch::{Observer, follow};
