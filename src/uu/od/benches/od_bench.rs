// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_od::uumain;
use uucore::benchmark::{get_bench_args, setup_test_file};

/// Size of the input dumped by each benchmark. od writes several bytes of
/// output per input byte, so this stays modest to keep the run times sane.
const INPUT_SIZE: usize = 1024 * 1024;

/// Generate binary input covering the whole byte range, so that the printable
/// character lookups and the number formatting both see every possible value.
fn binary_data() -> Vec<u8> {
    (0..INPUT_SIZE).map(|i| (i % 256) as u8).collect()
}

/// Benchmark the default output format (octal 2-byte words).
#[divan::bench]
fn od_default(bencher: Bencher) {
    let file_path = setup_test_file(&binary_data());

    bencher
        .with_inputs(|| get_bench_args(&[&file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark single byte hexadecimal output, the most common invocation.
#[divan::bench]
fn od_hex_bytes(bencher: Bencher) {
    let file_path = setup_test_file(&binary_data());

    bencher
        .with_inputs(|| get_bench_args(&[&"-t", &"x1", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark named character output, which escapes control characters.
#[divan::bench]
fn od_chars(bencher: Bencher) {
    let file_path = setup_test_file(&binary_data());

    bencher
        .with_inputs(|| get_bench_args(&[&"-c", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
