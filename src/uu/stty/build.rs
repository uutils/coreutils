// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[path = "../../../cfg_aliases.rs"]
mod cfg_aliases;

fn main() {
    cfg_aliases::register();
}
