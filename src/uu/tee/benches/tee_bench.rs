use divan::{Bencher, black_box};
use std::process::Command;
use uu_tee::uumain;
use uucore::benchmark::run_util_function;

#[divan::bench(args = ["10KB", "100MB"])]
fn tee_file(bencher: Bencher, size: &str) {
    Command::new("truncate")
        .args(["-s", size, "in"])
        .status()
        .expect("truncate failed");
    let args = &["/dev/null"];
    let stdin = Some("in");
    bencher.bench(|| {
        black_box(run_util_function(uumain, args, stdin));
    });
}

fn main() {
    divan::main();
}
