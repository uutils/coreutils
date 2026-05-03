use divan::{Bencher, black_box};
use uu_cat::uumain;
use uucore::benchmark::{run_util_function, setup_test_file};
#[divan::bench(args = [10_000, 10_000_000])]
fn cat_default(bencher: Bencher, size_bytes: usize) {
    let data = vec![b'a'; size_bytes];

    let file_path = setup_test_file(&data);
    let path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[path_str]));
    });
}

#[cfg(target_os = "linux")]
#[divan::bench(args = [10_000, 10_000_000])]
fn cat_stdin(bencher: Bencher, size_bytes: usize) {
    let data = vec![b'a'; size_bytes];
    let file_path = setup_test_file(&data);
    let file = std::fs::File::open(file_path).unwrap();
    let stdin_bak = rustix::io::dup(rustix::stdio::stdin()).unwrap();

    bencher.bench_local(|| {
        use rustix::stdio::dup2_stdin;
        rustix::fs::seek(&file, rustix::fs::SeekFrom::Start(0)).unwrap();
        dup2_stdin(&file).unwrap(); // should be 1 thread
        black_box(run_util_function(uumain, &[]));
        dup2_stdin(&stdin_bak).unwrap(); // should be 1 thread
    });
}

fn main() {
    divan::main();
}
