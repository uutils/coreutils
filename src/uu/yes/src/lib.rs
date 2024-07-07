// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod uu_args;
pub mod yes;
pub use uu_args::uu_app;

pub use yes::uumain;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod splice;
