use std::env;
use std::io::Write;
use std::fs::File;
use std::path::Path;

static TEMPLATE: &'static str = r#"
extern crate uu_@UTIL_CRATE@;
#[macro_use]
extern crate uucore;

use std::borrow::Cow;
use std::env;
use std::io::{self, Write};
use uu_@UTIL_CRATE@::UTILITY;
use uucore::{ProgramInfo, Util};

fn main() {
    //uucore::panic::install_sigpipe_hook();

    let stdin_raw = io::stdin();
    let stdout_raw = io::stdout();
    let stderr_raw = io::stderr();
    let stdin = stdin_raw.lock();
    let stdout = stdout_raw.lock();
    let stderr = stderr_raw.lock();

    let posix = env::var("POSIXLY_CORRECT").is_ok();

    let execpath;
    let name = match env::current_exe() {
        Ok(path) => {
            execpath = Some(path);
            execpath.as_ref().unwrap().file_stem().map(|stem| stem.to_string_lossy()).unwrap_or(Cow::from(executable!()))
        }
        Err(_) => Cow::from(executable!())
    };

    let mut pio = ProgramInfo::new(stdin, stdout, stderr, posix, name);
    let code = UTILITY.entry(std::env::args().collect(), &mut pio);
    // Since stdout is line-buffered by default, we need to ensure any pending
    // writes are flushed before exiting. Ideally, this should be enforced by
    // each utility.
    //
    // See: https://github.com/rust-lang/rust/issues/23818
    //
    let _ = pio.stdout.flush();
    std::process::exit(code);
}
"#;

pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let pkgname = env::var("CARGO_PKG_NAME").unwrap();

    let main = TEMPLATE.replace("@UTIL_CRATE@", &pkgname);
    let mut file = File::create(&Path::new(&out_dir).join("main.rs")).unwrap();

    write!(file, "{}", main).unwrap();
}
