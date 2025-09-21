#[cfg(target_os = "linux")]
uucore::bin!(uu_chcon);

#[cfg(not(target_os = "linux"))]
fn main() {}
