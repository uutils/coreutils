// spell-checker:ignore (ToDO) libstdbuf

extern crate cpp_build;

use cpp_build::Config;

fn main() {
    Config::new().pic(true).build("src/libstdbuf.rs");
}
