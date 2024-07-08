// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod cat;
pub mod uu_args;
pub use uu_args::*;

pub use cat::uumain;
pub use cat::CatResult;
pub use cat::FdReadable;
pub use cat::InputHandle;

/// Linux splice support
#[cfg(any(target_os = "linux", target_os = "android"))]
mod splice;
