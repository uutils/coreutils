// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use cfg_aliases::cfg_aliases;

pub fn main() {
    cfg_aliases! {
        acl: { feature = "feat_acl" },
        selinux: { all(feature = "feat_selinux", any(target_os = "android", target_os = "linux")) },
    }
}
