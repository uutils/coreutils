// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod tail;
pub mod uu_args;
pub use uu_args::options;
pub use uu_args::uu_app;

pub use tail::uumain;

pub mod args;
pub mod chunks;
mod follow;
mod parse;
mod paths;
mod platform;
pub mod text;
