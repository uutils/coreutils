#[cfg(feature = "cli-parser")]
uucore::bin!(uu_cp);

// for avoiding cargo check error
// consider adding a `main` function to `src/uu/cp/src/main.rs`
#[cfg(not(feature = "cli-parser"))]
fn main() {}
