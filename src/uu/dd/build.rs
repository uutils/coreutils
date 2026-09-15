// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

fn main() {
    if rustc_version::version_meta().unwrap().channel == rustc_version::Channel::Nightly {
        println!("cargo:rustc-cfg=nightly");
    }
}
