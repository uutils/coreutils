// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore aeiou

//! Benchmarks for `tr`.
//!
//! `tr` only reads stdin, so each bench redirects fd 0 onto a prepared
//! input file before invoking `uumain`. fd 1 is redirected to /dev/null
//! so the translated output does not flood the harness's terminal.
//! Both fds are restored after each iteration.

#[cfg(unix)]
use divan::{Bencher, black_box};
#[cfg(unix)]
use uu_tr::uumain;
#[cfg(unix)]
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

#[cfg(unix)]
fn bench_tr_with_stdin(bencher: Bencher, data: &[u8], args: &[&str]) {
    let file_path = setup_test_file(data);
    let file = std::fs::File::open(file_path).unwrap();
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .unwrap();
    let stdin_bak = rustix::io::dup(rustix::stdio::stdin()).unwrap();
    let stdout_bak = rustix::io::dup(rustix::stdio::stdout()).unwrap();

    bencher.bench_local(|| {
        use rustix::stdio::{dup2_stdin, dup2_stdout};
        rustix::fs::seek(&file, rustix::fs::SeekFrom::Start(0)).unwrap();
        dup2_stdin(&file).unwrap();
        dup2_stdout(&devnull).unwrap();
        black_box(run_util_function(uumain, args));
        dup2_stdin(&stdin_bak).unwrap();
        dup2_stdout(&stdout_bak).unwrap();
    });
}

#[cfg(unix)]
const SIZE_MB: usize = 16;

/// ASCII lowercase->uppercase range translation.
/// Exercises the AVX2 ASCII-range fast path on x86_64 hosts that
/// support it, and the scalar range fallback on other targets.
#[cfg(unix)]
#[divan::bench]
fn tr_ascii_range_lower_to_upper(bencher: Bencher) {
    let data = text_data::generate_by_size(SIZE_MB, 80);
    bench_tr_with_stdin(bencher, &data, &["a-z", "A-Z"]);
}

/// Single-character replacement. Exercises the existing
/// `process_single_char_replace` SIMD path; guards against
/// regressions outside the new range fast path.
#[cfg(unix)]
#[divan::bench]
fn tr_single_char_replace(bencher: Bencher) {
    let data = text_data::generate_by_size(SIZE_MB, 80);
    bench_tr_with_stdin(bencher, &data, &["a", "b"]);
}

/// Multi-character set translation. Falls through to the
/// 256-byte translation table path (no fast path applies).
#[cfg(unix)]
#[divan::bench]
fn tr_multi_char_translate(bencher: Bencher) {
    let data = text_data::generate_by_size(SIZE_MB, 80);
    bench_tr_with_stdin(bencher, &data, &["aeiou", "AEIOU"]);
}

/// Delete an ASCII range — covers the deletion path.
#[cfg(unix)]
#[divan::bench]
fn tr_delete_ascii_range(bencher: Bencher) {
    let data = text_data::generate_by_size(SIZE_MB, 80);
    bench_tr_with_stdin(bencher, &data, &["-d", "a-z"]);
}

fn main() {
    divan::main();
}
