// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        // Where uucore's safe_traversal, and so `DirAnchor`, exist.
        has_safe_traversal: { all(unix, not(target_os = "redox")) },
    };
}
