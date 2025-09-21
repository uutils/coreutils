#[cfg(target_os = "linux")]
uucore::bin!(uu_runcon);

#[cfg(not(target_os = "linux"))]
fn main() {}
