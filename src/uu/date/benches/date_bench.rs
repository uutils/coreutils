// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::io::Write;
use tempfile::NamedTempFile;
use uu_date::uumain;
use uucore::benchmark::run_util_function;

/// Helper to create a temporary file containing N lines of date strings.
fn setup_date_file(lines: usize, date_format: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for _ in 0..lines {
        writeln!(file, "{date_format}").unwrap();
    }
    file
}

/// Benchmarks processing a file containing simple ISO dates.
#[divan::bench]
fn file_iso_dates(bencher: Bencher) {
    let count = 1_000;
    let file = setup_date_file(count, "2023-05-10 12:00:00");
    let path = file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-f", path]));
    });
}

/// Benchmarks processing a file containing dates with Timezone abbreviations.
#[divan::bench]
fn file_tz_abbreviations(bencher: Bencher) {
    let count = 1_000;
    // "EST" triggers the abbreviation lookup and double-parsing logic
    let file = setup_date_file(count, "2023-05-10 12:00:00 EST");
    let path = file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-f", path]));
    });
}

/// Benchmarks formatting speed using a custom output format.
#[divan::bench]
fn file_custom_format(bencher: Bencher) {
    let count = 1_000;
    let file = setup_date_file(count, "2023-05-10 12:00:00");
    let path = file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-f", path, "+%A %d %B %Y"]));
    });
}

/// Benchmarks the overhead of starting the utility for a single date (no file).
#[divan::bench]
fn single_date_now(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &[]));
    });
}

/// Benchmarks parsing a complex relative date string passed as an argument.
#[divan::bench]
fn complex_relative_date(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--date=last friday 12:00 + 2 days"],
        ));
    });
}

fn main() {
    divan::main();
}
