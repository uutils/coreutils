// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_nl::uumain;
use uucore::benchmark::{create_test_file, run_util_function, text_data};

/// Benchmark numbering many lines (default mode - most common use case)
#[divan::bench(args = [100_000])]
fn nl_many_lines(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

/// Benchmark large file with -ba option (number all lines - most common argument)
#[divan::bench(args = [10])]
fn nl_large_file(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_size(size_mb, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-ba", file_path_str]));
    });
}

fn main() {
    divan::main();
}
