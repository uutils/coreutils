// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! This package is specific to Android and some Linux distributions. On other
//! targets, provide a stub main to keep the binary target present and the
//! workspace buildable. Using item-level cfg avoids excluding the crate
//! entirely (via #![cfg(...)]), which can break tooling and cross builds that
//! expect this binary to exist even when it's a no-op off Linux.

#[cfg(any(target_os = "linux", target_os = "android"))]
uucore::bin!(uu_chcon);

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn main() {
    eprintln!("chcon: SELinux is not supported on this platform");
    std::process::exit(1);
}
