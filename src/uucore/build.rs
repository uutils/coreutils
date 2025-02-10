// spell-checker:ignore libsystemd
fn main() {
    #[cfg(target_os = "linux")]
    #[cfg(feature = "uptime")]
    pkg_config::find_library("libsystemd").unwrap();
}
