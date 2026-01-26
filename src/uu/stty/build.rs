use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        bsd: { any(
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "ios",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "openbsd"
        ) },
    }
}
