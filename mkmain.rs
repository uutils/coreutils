use std::env;
use std::io::Write;
use std::fs::File;
use std::path::Path;

static TEMPLATE: &'static str = "\
extern crate uu_@UTIL_CRATE@;
extern crate uucore;

use std::io::Write;
use uu_@UTIL_CRATE@::uumain;

fn main() {
    uucore::panic::install_sigpipe_hook();

    let code = uumain(uucore::args().collect());
    // Since stdout is line-buffered by default, we need to ensure any pending
    // writes are flushed before exiting. Ideally, this should be enforced by
    // each utility.
    //
    // See: https://github.com/rust-lang/rust/issues/23818
    //
    std::io::stdout().flush().expect(\"could not flush stdout\");
    std::process::exit(code);
}
";

pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let pkgname = env::var("CARGO_PKG_NAME").unwrap();

    let main = TEMPLATE.replace("@UTIL_CRATE@", &pkgname);
    let mut file = File::create(&Path::new(&out_dir).join("main.rs")).unwrap();

    write!(file, "{}", main).unwrap();
}
