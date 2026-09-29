#[cfg(unix)]
use divan::{Bencher, black_box};
#[cfg(unix)]
use uu_tee::uumain;
#[cfg(unix)]
use uucore::benchmark::{get_bench_args, setup_test_file};

#[cfg(unix)]
#[divan::bench(args = [10_000_000])]
fn tee_stdin_file(bencher: Bencher, size_bytes: usize) {
    let data = vec![b'a'; size_bytes];
    let file_path = setup_test_file(&data);
    let file = std::fs::File::open(file_path).unwrap();
    let stdin_bak = rustix::io::dup(rustix::stdio::stdin()).unwrap();
    rustix::stdio::dup2_stdin(&file).unwrap(); // should be 1 thread

    bencher
        .with_inputs(|| {
            rustix::fs::seek(&file, rustix::fs::SeekFrom::Start(0)).unwrap();
            get_bench_args(&[]).into_iter()
        })
        .bench_local_values(|args| black_box(uumain(args)));

    rustix::stdio::dup2_stdin(&stdin_bak).unwrap(); // should be 1 thread
}

fn main() {
    // Rewind happens before each sample; force one iteration after CLI/env overrides.
    #[cfg(not(codspeed))]
    divan::Divan::from_args().sample_size(1).main();
    #[cfg(codspeed)]
    divan::main();
}
