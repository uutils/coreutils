// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        // Platforms where ls can show an ACL/`+` indicator (xattrs or macOS ACL API).
        acl_platform: { any(
            target_os = "freebsd",
            target_os = "hurd",
            target_os = "linux",
            target_os = "macos",
            target_os = "netbsd"
        ) },
        // Platforms where one listxattr pass covers both ACL/`+` and capability color.
        xattr_summary: { any(
            target_os = "freebsd",
            target_os = "hurd",
            target_os = "linux",
            target_os = "netbsd"
        ) },
    };
}
