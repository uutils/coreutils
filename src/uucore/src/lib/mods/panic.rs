// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
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
use std::panic::{self, PanicHookInfo};

/// Decide whether a panic was caused by a broken pipe (SIGPIPE) error.
fn is_broken_pipe(info: &PanicHookInfo) -> bool {
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

/// Preserve inherited SIGPIPE settings from parent process.
///
/// Rust unconditionally sets SIGPIPE to SIG_IGN on startup. This function
/// checks if the parent process (e.g., `env --default-signal=PIPE`) intended
/// for SIGPIPE to be set to default by checking the RUST_SIGPIPE environment
/// variable. If set to "default", it restores SIGPIPE to SIG_DFL.
#[cfg(unix)]
pub fn preserve_inherited_sigpipe() {
    use nix::libc;

    // Check if parent specified that SIGPIPE should be default
    if let Ok(val) = std::env::var("RUST_SIGPIPE") {
        if val == "default" {
            unsafe {
                libc::signal(libc::SIGPIPE, libc::SIG_DFL);
                // Remove the environment variable so child processes don't inherit it incorrectly
                std::env::remove_var("RUST_SIGPIPE");
            }
        }
    }
}

#[cfg(not(unix))]
pub fn preserve_inherited_sigpipe() {
    // No-op on non-Unix platforms
}
