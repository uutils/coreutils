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
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut args: Vec<String> = vec!["--to=si".to_string()];
            args.extend(numbers);
            args
        })
        .bench_values(|args| {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            black_box(run_util_function(uumain, &arg_refs));
        });
}

/// Benchmark SI formatting with precision format
#[divan::bench(args = [10_000])]
fn numfmt_to_si_precision(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut args: Vec<String> = vec!["--to=si".to_string(), "--format=%.6f".to_string()];
            args.extend(numbers);
            args
        })
        .bench_values(|args| {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            black_box(run_util_function(uumain, &arg_refs));
        });
}

/// Benchmark IEC (binary) formatting
#[divan::bench(args = [10_000])]
fn numfmt_to_iec(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut args: Vec<String> = vec!["--to=iec".to_string()];
            args.extend(numbers);
            args
        })
        .bench_values(|args| {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            black_box(run_util_function(uumain, &arg_refs));
        });
}

/// Benchmark parsing from SI format back to raw numbers
#[divan::bench(args = [10_000])]
fn numfmt_from_si(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| {
            // Generate SI formatted data (e.g., "1K", "2K", etc.)
            let numbers: Vec<String> = (1..=count).map(|n| format!("{n}K")).collect();
            let mut args: Vec<String> = vec!["--from=si".to_string()];
            args.extend(numbers);
            args
        })
        .bench_values(|args| {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            black_box(run_util_function(uumain, &arg_refs));
        });
}

/// Benchmark large numbers with SI formatting
#[divan::bench(args = [10_000])]
fn numfmt_large_numbers_si(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| {
            // Generate numbers that all produce uniform SI output lengths (all in 1-9M range)
            // This avoids variance from variable output string lengths
            let numbers: Vec<String> = (1..=count)
                .map(|n| ((n % 9) + 1) * 1_000_000)
                .map(|n| n.to_string())
                .collect();
            let mut args: Vec<String> = vec!["--to=si".to_string()];
            args.extend(numbers);
            args
        })
        .bench_values(|args| {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            black_box(run_util_function(uumain, &arg_refs));
        });
}

/// Benchmark different padding widths
#[divan::bench(args = [(10_000, 50)])]
fn numfmt_padding(bencher: Bencher, (count, padding): (usize, usize)) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut args: Vec<String> = vec!["--to=si".to_string(), format!("--padding={padding}")];
            args.extend(numbers);
            args
        })
        .bench_values(|args| {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            black_box(run_util_function(uumain, &arg_refs));
        });
}

/// Benchmark round modes with SI formatting
#[divan::bench(args = [("up", 10_000), ("down", 10_000), ("towards-zero", 10_000)])]
fn numfmt_round_modes(bencher: Bencher, (round_mode, count): (&str, usize)) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut args: Vec<String> =
                vec!["--to=si".to_string(), format!("--round={round_mode}")];
            args.extend(numbers);
            args
        })
        .bench_values(|args| {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            black_box(run_util_function(uumain, &arg_refs));
        });
}

fn main() {
    divan::main();
}
