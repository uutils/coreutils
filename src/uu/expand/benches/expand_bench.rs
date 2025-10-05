// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fmt::Write;
use uu_expand::uumain;
use uucore::benchmark::{create_test_file, run_util_function};

/// Helper function to run expand benchmark with generated data
fn bench_expand(bencher: Bencher, data: impl AsRef<[u8]>, args: &[&str]) {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_test_file(data.as_ref(), temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    let mut all_args = vec![];
    all_args.extend_from_slice(args);
    all_args.push(file_path_str);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &all_args));
    });
}

/// Benchmark expanding tabs on files with many short lines
#[divan::bench(args = [100_000])]
fn expand_many_lines(bencher: Bencher, num_lines: usize) {
    let data = (0..num_lines).fold(String::new(), |mut acc, i| {
        writeln!(&mut acc, "line{i}\tvalue{}\tdata{}", i * 2, i * 3).unwrap();
        acc
    });
    bench_expand(bencher, data, &[]);
}

/// Benchmark expanding tabs with custom tab stops
#[divan::bench(args = [50_000])]
fn expand_custom_tabstops(bencher: Bencher, num_lines: usize) {
    let data = (0..num_lines).fold(String::new(), |mut acc, i| {
        writeln!(&mut acc, "a\tb\tc\td\te{i}").unwrap();
        acc
    });
    bench_expand(bencher, data, &["--tabs=4,8,12"]);
}

fn main() {
    divan::main();
}
