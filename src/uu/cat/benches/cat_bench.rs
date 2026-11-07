use divan::{Bencher, black_box};
use uu_cat::uumain;
use uucore::benchmark::{get_bench_args, setup_test_file};

#[divan::bench(args = [10_000, 10_000_000])]
fn cat_default(bencher: Bencher, size_bytes: usize) {
    let data = vec![b'a'; size_bytes];

    let file_path = setup_test_file(&data);
    let path_str = file_path.to_str().unwrap();
    let args = get_bench_args(&[&path_str]);

    bencher
        .with_inputs(|| args.clone().into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
