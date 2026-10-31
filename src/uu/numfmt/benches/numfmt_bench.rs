// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Benchmarks for `numfmt`, with the numbers as arguments, and on stdin.

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

/// Run `uumain` with `args`, `data` on stdin and the output thrown away.
#[cfg(unix)]
fn bench_with_stdin(bencher: Bencher, data: &[u8], args: &[&str]) {
    use rustix::stdio::{dup2_stdin, dup2_stdout};

    let file = std::fs::File::open(uucore::benchmark::setup_test_file(data)).unwrap();
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .unwrap();
    let stdin_bak = rustix::io::dup(rustix::stdio::stdin()).unwrap();
    let stdout_bak = rustix::io::dup(rustix::stdio::stdout()).unwrap();

    bencher.bench_local(|| {
        rustix::fs::seek(&file, rustix::fs::SeekFrom::Start(0)).unwrap();
        dup2_stdin(&file).unwrap();
        dup2_stdout(&devnull).unwrap();
        black_box(run_util_function(uumain, args));
        dup2_stdin(&stdin_bak).unwrap();
        dup2_stdout(&stdout_bak).unwrap();
    });
}

/// Benchmark SI formatting with the numbers on stdin
#[cfg(unix)]
#[divan::bench]
fn numfmt_stream_to_si(bencher: Bencher) {
    let data: Vec<u8> = (1..=100_000u64)
        .flat_map(|n| format!("{}\n", n * 7919).into_bytes())
        .collect();
    bench_with_stdin(bencher, &data, &["--to=si"]);
}

/// The same, with a precision: formats through the exact float path
#[cfg(unix)]
#[divan::bench]
fn numfmt_stream_to_si_precision(bencher: Bencher) {
    let data: Vec<u8> = (1..=100_000u64)
        .flat_map(|n| format!("{}\n", n * 7919).into_bytes())
        .collect();
    bench_with_stdin(bencher, &data, &["--to=si", "--format=%.2f"]);
}

fn main() {
    divan::main();
}
