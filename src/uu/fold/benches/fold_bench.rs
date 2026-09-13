// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_fold::uumain;
use uucore::benchmark::{create_test_file, get_bench_args};

/// Benchmark folding many short lines
#[divan::bench(args = [100_000])]
fn fold_many_lines(bencher: Bencher, num_lines: usize) {
    let mut data = String::with_capacity(num_lines * 110);
    for i in 0..num_lines {
        data.push_str("This is a very long line number ");
        append_usize(&mut data, i);
        data.push_str(" that definitely needs to be folded at the default width of 80 columns\n");
    }
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(data.as_bytes(), temp_dir.path());

    bencher
        .with_inputs(|| get_bench_args(&[&file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark folding with custom width
#[divan::bench(args = [50_000])]
fn fold_custom_width(bencher: Bencher, num_lines: usize) {
    let mut data = String::with_capacity(num_lines * 80);
    for i in 0..num_lines {
        data.push_str("Line ");
        append_usize(&mut data, i);
        data.push_str(" with enough text to exceed width 40 characters and require folding\n");
    }
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(data.as_bytes(), temp_dir.path());

    bencher
        .with_inputs(|| get_bench_args(&[&"-w", &"40", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}

fn append_usize(buf: &mut String, mut value: usize) {
    let mut digits = [0u8; 20];
    let mut idx = digits.len();

    if value == 0 {
        buf.push('0');
        return;
    }

    while value > 0 {
        idx -= 1;
        digits[idx] = b'0' + (value % 10) as u8;
        value /= 10;
    }

    buf.push_str(std::str::from_utf8(&digits[idx..]).unwrap());
}
