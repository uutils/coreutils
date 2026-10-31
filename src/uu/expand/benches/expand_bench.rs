// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fmt::Write;
use tempfile::TempDir;
use uu_expand::uumain;
use uucore::benchmark::{create_test_file, get_bench_args};

/// Benchmark expanding tabs on files with many short lines
#[divan::bench(args = [100_000])]
fn expand_many_lines(bencher: Bencher, num_lines: usize) {
    let data = (0..num_lines).fold(String::new(), |mut acc, i| {
        writeln!(&mut acc, "line{i}\tvalue{}\tdata{}", i * 2, i * 3).unwrap();
        acc
    });
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(data.as_ref(), temp_dir.path());

    bencher
        .with_inputs(|| get_bench_args(&[&file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark expanding tabs with custom tab stops
#[divan::bench(args = [50_000])]
fn expand_custom_tabstops(bencher: Bencher, num_lines: usize) {
    let data = (0..num_lines).fold(String::new(), |mut acc, i| {
        writeln!(&mut acc, "a\tb\tc\td\te{i}").unwrap();
        acc
    });
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(data.as_ref(), temp_dir.path());

    bencher
        .with_inputs(|| get_bench_args(&[&"--tabs=4,8,12", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
