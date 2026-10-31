// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub fn register() {
    cfg_aliases::cfg_aliases! {
        bsd: { any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "netbsd",
            target_os = "openbsd"
        ) },
        has_o_path: { any(target_os = "linux", target_os = "android") },
        has_o_search: { any(
            target_os = "macos",
            target_os = "ios",
            target_os = "tvos",
            target_os = "watchos",
            target_os = "visionos",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "illumos",
            target_os = "solaris"
        ) },
        selinux: { all(feature = "selinux", any(target_os = "android", target_os = "linux")) },
        smack: { all(feature = "smack", target_os = "linux") },
    }
}
