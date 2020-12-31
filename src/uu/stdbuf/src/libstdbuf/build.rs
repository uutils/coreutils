// spell-checker:ignore (ToDO) libstdbuf

use cpp_build::Config;

fn main() {
    Config::new().pic(true).build("src/libstdbuf.rs");
}
