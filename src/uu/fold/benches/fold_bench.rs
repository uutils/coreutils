// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fmt::Write;
use uu_fold::uumain;
use uucore::benchmark::{create_test_file, run_util_function};

/// Benchmark folding many short lines
#[divan::bench(args = [100_000])]
fn fold_many_lines(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    // Create long lines that need folding
    let data = (0..num_lines)
        .fold(String::new(), |mut acc, i| {
            writeln!(&mut acc, "This is a very long line number {i} that definitely needs to be folded at the default width of 80 columns").unwrap();
            acc
        });
    let file_path = create_test_file(data.as_bytes(), temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

/// Benchmark folding with custom width
#[divan::bench(args = [50_000])]
fn fold_custom_width(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = (0..num_lines).fold(String::new(), |mut acc, i| {
        writeln!(
            &mut acc,
            "Line {i} with enough text to exceed width 40 characters and require folding"
        )
        .unwrap();
        acc
    });
    let file_path = create_test_file(data.as_bytes(), temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-w", "40", file_path_str]));
    });
}

fn main() {
    divan::main();
}
