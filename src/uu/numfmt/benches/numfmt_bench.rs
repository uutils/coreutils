// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_numfmt::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark SI formatting with different number counts
#[divan::bench(args = [1_000_000])]
fn numfmt_to_si(bencher: Bencher, count: usize) {
    let data = text_data::generate_numbers(count);
    let file_path = setup_test_file(data.as_bytes());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["--to=si", file_path_str]));
    });
}

/// Benchmark SI formatting with precision format
#[divan::bench(args = [1_000_000])]
fn numfmt_to_si_precision(bencher: Bencher, count: usize) {
    let data = text_data::generate_numbers(count);
    let file_path = setup_test_file(data.as_bytes());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--to=si", "--format=%.6f", file_path_str],
        ));
    });
}

/// Benchmark IEC (binary) formatting
#[divan::bench(args = [1_000_000])]
fn numfmt_to_iec(bencher: Bencher, count: usize) {
    let data = text_data::generate_numbers(count);
    let file_path = setup_test_file(data.as_bytes());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["--to=iec", file_path_str]));
    });
}

/// Benchmark parsing from SI format back to raw numbers
#[divan::bench(args = [1_000_000])]
fn numfmt_from_si(bencher: Bencher, count: usize) {
    // Generate SI formatted data (e.g., "1.0K", "2.0K", etc.)
    let data = (1..=count)
        .map(|n| format!("{:.1}K", n as f64 / 1000.0))
        .collect::<Vec<_>>()
        .join("\n");
    let file_path = setup_test_file(data.as_bytes());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["--from=si", file_path_str]));
    });
}

/// Benchmark large numbers with SI formatting
#[divan::bench(args = [1_000_000])]
fn numfmt_large_numbers_si(bencher: Bencher, count: usize) {
    // Generate larger numbers (millions to billions range)
    let data = (1..=count)
        .map(|n| (n * 1_000_000).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let file_path = setup_test_file(data.as_bytes());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["--to=si", file_path_str]));
    });
}

/// Benchmark different padding widths
#[divan::bench(args = [(1_000_000, 5), (1_000_000, 50)])]
fn numfmt_padding(bencher: Bencher, (count, padding): (usize, usize)) {
    let data = text_data::generate_numbers(count);
    let file_path = setup_test_file(data.as_bytes());
    let file_path_str = file_path.to_str().unwrap();
    let padding_arg = format!("--padding={padding}");

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--to=si", &padding_arg, file_path_str],
        ));
    });
}

/// Benchmark round modes with SI formatting
#[divan::bench(args = [("up", 100_000), ("down", 1_000_000), ("towards-zero", 1_000_000)])]
fn numfmt_round_modes(bencher: Bencher, (round_mode, count): (&str, usize)) {
    let data = text_data::generate_numbers(count);
    let file_path = setup_test_file(data.as_bytes());
    let file_path_str = file_path.to_str().unwrap();
    let round_arg = format!("--round={round_mode}");

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--to=si", &round_arg, file_path_str],
        ));
    });
}

fn main() {
    divan::main();
}
