// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_numfmt::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark SI formatting by passing numbers as command-line arguments
#[divan::bench(args = [10_000])]
fn numfmt_to_si(bencher: Bencher, count: usize) {
    let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
    let mut args = vec!["--to=si"];
    let number_refs: Vec<&str> = numbers.iter().map(|s| s.as_str()).collect();
    args.extend(number_refs);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark SI formatting with precision format
#[divan::bench(args = [10_000])]
fn numfmt_to_si_precision(bencher: Bencher, count: usize) {
    let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
    let mut args = vec!["--to=si", "--format=%.6f"];
    let number_refs: Vec<&str> = numbers.iter().map(|s| s.as_str()).collect();
    args.extend(number_refs);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark IEC (binary) formatting
#[divan::bench(args = [10_000])]
fn numfmt_to_iec(bencher: Bencher, count: usize) {
    let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
    let mut args = vec!["--to=iec"];
    let number_refs: Vec<&str> = numbers.iter().map(|s| s.as_str()).collect();
    args.extend(number_refs);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark parsing from SI format back to raw numbers
#[divan::bench(args = [10_000])]
fn numfmt_from_si(bencher: Bencher, count: usize) {
    // Generate SI formatted data (e.g., "1K", "2K", etc.)
    let numbers: Vec<String> = (1..=count).map(|n| format!("{n}K")).collect();
    let mut args = vec!["--from=si"];
    let number_refs: Vec<&str> = numbers.iter().map(|s| s.as_str()).collect();
    args.extend(number_refs);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark large numbers with SI formatting
#[divan::bench(args = [10_000])]
fn numfmt_large_numbers_si(bencher: Bencher, count: usize) {
    // Generate larger numbers (millions to billions range)
    let numbers: Vec<String> = (1..=count).map(|n| (n * 1_000_000).to_string()).collect();
    let mut args = vec!["--to=si"];
    let number_refs: Vec<&str> = numbers.iter().map(|s| s.as_str()).collect();
    args.extend(number_refs);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark different padding widths
#[divan::bench(args = [(10_000, 50)])]
fn numfmt_padding(bencher: Bencher, (count, padding): (usize, usize)) {
    let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
    let padding_arg = format!("--padding={padding}");
    let mut args = vec!["--to=si", &padding_arg];
    let number_refs: Vec<&str> = numbers.iter().map(|s| s.as_str()).collect();
    args.extend(number_refs);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark round modes with SI formatting
#[divan::bench(args = [("up", 10_000), ("down", 10_000), ("towards-zero", 10_000)])]
fn numfmt_round_modes(bencher: Bencher, (round_mode, count): (&str, usize)) {
    let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
    let round_arg = format!("--round={round_mode}");
    let mut args = vec!["--to=si", &round_arg];
    let number_refs: Vec<&str> = numbers.iter().map(|s| s.as_str()).collect();
    args.extend(number_refs);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

fn main() {
    divan::main();
}
