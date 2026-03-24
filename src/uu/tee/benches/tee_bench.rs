use divan::{Bencher, black_box};
use std::process::Command;
use uu_tee::uumain;
use uucore::benchmark::{run_util_function, set_stdin};

#[divan::bench(args = ["10KB", "100MB"])]
fn tee_file(bencher: Bencher, size: &str) {
    Command::new("truncate")
        .args(["-s", size, "in"])
        .status()
        .expect("truncate failed");
    let args = &["/dev/null"];
    bencher.bench(|| {
		set_stdin("in");
        black_box(run_util_function(uumain, args));
    });
}
