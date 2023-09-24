// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) libstdbuf

use cpp_build::Config;

fn main() {
    Config::new().pic(true).build("src/libstdbuf.rs");
}
