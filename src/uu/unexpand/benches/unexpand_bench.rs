// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_unexpand::uumain;
use uucore::benchmark::{create_test_file, run_util_function};

/// Generate text data with leading spaces (typical unexpand use case)
fn generate_indented_text(num_lines: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..num_lines {
        // Add varying amounts of leading spaces (4, 8, 12, etc.)
        let indent = (i % 4 + 1) * 4;
        data.extend(vec![b' '; indent]);
        data.extend_from_slice(b"This is a line of text with leading spaces\n");
    }
    data
}

/// Benchmark unexpanding many lines with leading spaces (most common use case)
#[divan::bench(args = [100_000])]
fn unexpand_many_lines(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_indented_text(num_lines);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

/// Benchmark large file with spaces (tests performance on large files)
#[divan::bench(args = [10])]
fn unexpand_large_file(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();

    // Generate approximately size_mb worth of indented lines
    let line_size = 50; // approximate bytes per line
    let num_lines = (size_mb * 1024 * 1024) / line_size;
    let data = generate_indented_text(num_lines);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

fn main() {
    divan::main();
}
