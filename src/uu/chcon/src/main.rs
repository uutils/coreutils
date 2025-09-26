// On non-Linux targets, provide a stub main to keep the binary target present
// and the workspace buildable. Using item-level cfg avoids excluding the crate
// entirely (via #![cfg(...)]), which can break tooling and cross builds that
// expect this binary to exist even when it’s a no-op off Linux.
#[cfg(target_os = "linux")]
uucore::bin!(uu_chcon);

#[cfg(not(target_os = "linux"))]
fn main() {}
