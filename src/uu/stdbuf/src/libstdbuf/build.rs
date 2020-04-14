extern crate cpp_build;

use cpp_build::Config;

fn main() {
    Config::new().pic(true).build("libstdbuf.rs");
}
