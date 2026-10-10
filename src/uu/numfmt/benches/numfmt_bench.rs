// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Benchmarks for `numfmt`, with the numbers as arguments, and on stdin.

use divan::{Bencher, black_box};
use uu_numfmt::uumain;
use uucore::benchmark::get_bench_args;

/// Benchmark SI formatting by passing numbers as command-line arguments
#[divan::bench(args = [10_000])]
fn numfmt_to_si(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut raw_args: Vec<String> = vec!["--to=si".to_string()];
            raw_args.extend(numbers);
            let args = raw_args
                .iter()
                .map(|arg| arg as &dyn AsRef<std::ffi::OsStr>)
                .collect::<Vec<_>>();
            get_bench_args(&args).into_iter()
        })
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark SI formatting with precision format
#[divan::bench(args = [10_000])]
fn numfmt_to_si_precision(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut raw_args: Vec<String> =
                vec!["--to=si".to_string(), "--format=%.6f".to_string()];
            raw_args.extend(numbers);
            let args = raw_args
                .iter()
                .map(|arg| arg as &dyn AsRef<std::ffi::OsStr>)
                .collect::<Vec<_>>();
            get_bench_args(&args).into_iter()
        })
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark IEC (binary) formatting
#[divan::bench(args = [10_000])]
fn numfmt_to_iec(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut raw_args: Vec<String> = vec!["--to=iec".to_string()];
            raw_args.extend(numbers);
            let args = raw_args
                .iter()
                .map(|arg| arg as &dyn AsRef<std::ffi::OsStr>)
                .collect::<Vec<_>>();
            get_bench_args(&args).into_iter()
        })
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark parsing from SI format back to raw numbers
#[divan::bench(args = [10_000])]
fn numfmt_from_si(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| {
            // Generate SI formatted data (e.g., "1K", "2K", etc.)
            let numbers: Vec<String> = (1..=count).map(|n| format!("{n}K")).collect();
            let mut raw_args: Vec<String> = vec!["--from=si".to_string()];
            raw_args.extend(numbers);
            let args = raw_args
                .iter()
                .map(|arg| arg as &dyn AsRef<std::ffi::OsStr>)
                .collect::<Vec<_>>();
            get_bench_args(&args).into_iter()
        })
        .bench_values(|args| black_box(uumain(args)));
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
            let mut raw_args: Vec<String> = vec!["--to=si".to_string()];
            raw_args.extend(numbers);
            let args = raw_args
                .iter()
                .map(|arg| arg as &dyn AsRef<std::ffi::OsStr>)
                .collect::<Vec<_>>();
            get_bench_args(&args).into_iter()
        })
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark different padding widths
#[divan::bench(args = [(10_000, 50)])]
fn numfmt_padding(bencher: Bencher, (count, padding): (usize, usize)) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut raw_args: Vec<String> =
                vec!["--to=si".to_string(), format!("--padding={padding}")];
            raw_args.extend(numbers);
            let args = raw_args
                .iter()
                .map(|arg| arg as &dyn AsRef<std::ffi::OsStr>)
                .collect::<Vec<_>>();
            get_bench_args(&args).into_iter()
        })
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark round modes with SI formatting
#[divan::bench(args = [("up", 10_000), ("down", 10_000), ("towards-zero", 10_000)])]
fn numfmt_round_modes(bencher: Bencher, (round_mode, count): (&str, usize)) {
    bencher
        .with_inputs(|| {
            let numbers: Vec<String> = (1..=count).map(|n| n.to_string()).collect();
            let mut raw_args: Vec<String> =
                vec!["--to=si".to_string(), format!("--round={round_mode}")];
            raw_args.extend(numbers);
            let args = raw_args
                .iter()
                .map(|arg| arg as &dyn AsRef<std::ffi::OsStr>)
                .collect::<Vec<_>>();
            get_bench_args(&args).into_iter()
        })
        .bench_values(|args| black_box(uumain(args)));
}

/// Run `uumain` with `args`, `data` on stdin and the output thrown away.
#[cfg(unix)]
fn bench_with_stdin(bencher: Bencher, data: &[u8], args: Vec<std::ffi::OsString>) {
    use rustix::stdio::{dup2_stdin, dup2_stdout};

    let file = std::fs::File::open(uucore::benchmark::setup_test_file(data)).unwrap();
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .unwrap();
    let stdin_bak = rustix::io::dup(rustix::stdio::stdin()).unwrap();
    let stdout_bak = rustix::io::dup(rustix::stdio::stdout()).unwrap();
    dup2_stdin(&file).unwrap();
    dup2_stdout(&devnull).unwrap();

    bencher
        .with_inputs(|| {
            rustix::fs::seek(&file, rustix::fs::SeekFrom::Start(0)).unwrap();
            args.clone().into_iter()
        })
        .bench_local_values(|args| black_box(uumain(args)));

    dup2_stdin(&stdin_bak).unwrap();
    dup2_stdout(&stdout_bak).unwrap();
}

/// Benchmark SI formatting with the numbers on stdin
#[cfg(unix)]
#[divan::bench]
fn numfmt_stream_to_si(bencher: Bencher) {
    let data: Vec<u8> = (1..=100_000u64)
        .flat_map(|n| format!("{}\n", n * 7919).into_bytes())
        .collect();
    bench_with_stdin(bencher, &data, get_bench_args(&[&"--to=si"]));
}

/// The same, with a precision: formats through the exact float path
#[cfg(unix)]
#[divan::bench]
fn numfmt_stream_to_si_precision(bencher: Bencher) {
    let data: Vec<u8> = (1..=100_000u64)
        .flat_map(|n| format!("{}\n", n * 7919).into_bytes())
        .collect();
    bench_with_stdin(
        bencher,
        &data,
        get_bench_args(&[&"--to=si", &"--format=%.2f"]),
    );
}

fn main() {
    // Rewind happens before each sample; force one iteration after CLI/env overrides.
    #[cfg(not(codspeed))]
    divan::Divan::from_args().sample_size(1).main();
    #[cfg(codspeed)]
    divan::main();
}
