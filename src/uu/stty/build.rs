// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

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
