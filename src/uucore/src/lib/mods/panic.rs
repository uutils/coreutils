//! Custom panic hooks that allow silencing certain types of errors.
//!
//! Use the [`mute_sigpipe_panic`] function to silence panics caused by
//! broken pipe errors. This can happen when a process is still
//! producing data when the consuming process terminates and closes the
//! pipe. For example,
//!
//! ```sh
//! $ seq inf | head -n 1
//! ```
//!
use std::panic;
use std::panic::PanicInfo;

/// Decide whether a panic was caused by a broken pipe (SIGPIPE) error.
fn is_broken_pipe(info: &PanicInfo) -> bool {
    if let Some(res) = info.payload().downcast_ref::<String>() {
        if res.contains("BrokenPipe") || res.contains("Broken pipe") {
            return true;
        }
    }
    false
}

/// Terminate without error on panics that occur due to broken pipe errors.
///
/// For background discussions on `SIGPIPE` handling, see
///
/// * `<https://github.com/uutils/coreutils/issues/374>`
/// * `<https://github.com/uutils/coreutils/pull/1106>`
/// * `<https://github.com/rust-lang/rust/issues/62569>`
/// * `<https://github.com/BurntSushi/ripgrep/issues/200>`
/// * `<https://github.com/crev-dev/cargo-crev/issues/287>`
///
pub fn mute_sigpipe_panic() {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if !is_broken_pipe(info) {
            hook(info);
        }
    }));
}
