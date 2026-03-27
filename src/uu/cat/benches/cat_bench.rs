use divan::{Bencher, black_box};
use uu_cat::uumain;
use uucore::benchmark::{run_util_function, setup_test_file};

#[divan::bench(args = [10_000])]
fn bench_cat_default(bencher: Bencher, size_bytes: usize) {
    let data = vec![b'a'; size_bytes];

    let file_path = setup_test_file(&data);
    let path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[path_str]));
    });
}

fn main() {
    divan::main();
}
