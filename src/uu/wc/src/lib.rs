// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod uu_args;
pub mod wc;
pub use uu_args::options;
pub use uu_args::uu_app;
pub use uu_args::ARG_FILES;
pub static STDIN_REPR: &str = "-";

pub use wc::uumain;

mod count_fast;
mod countable;
pub use countable::WordCountable;
mod utf8;
mod word_count;
