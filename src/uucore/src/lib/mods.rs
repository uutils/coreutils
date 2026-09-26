// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// mods ~ cross-platforms modules (core/bundler file)

pub mod args;
pub mod clap_localization;
pub mod display;
pub mod error;
#[cfg(all(feature = "fs", any(unix, windows, target_os = "wasi")))]
pub mod io;
pub mod line_ending;
pub mod locale;
pub mod os;
pub mod panic;
pub mod posix;
