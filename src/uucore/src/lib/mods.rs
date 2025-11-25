// mods ~ cross-platforms modules (core/bundler file)

pub mod clap_localization;
pub mod display;
pub mod error;
#[cfg(feature = "fs")]
pub mod io;
pub mod line_ending;
pub mod locale;
pub mod os;
pub mod panic;
pub mod posix;
