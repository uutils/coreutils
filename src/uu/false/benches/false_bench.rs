// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::process::Command;
use uu_false::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark the common case: false with no arguments
#[divan::bench]
fn false_no_args(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &[]));
    });
}

/// Benchmark false with arbitrary arguments (e.g., called via dummy symlink)
#[divan::bench]
fn false_with_args(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["some", "dummy", "args"]));
    });
}

/// Benchmark multiple consecutive invocations
#[divan::bench]
fn false_consecutive_calls(bencher: Bencher) {
    bencher.bench(|| {
        for _ in 0..100 {
            black_box(run_util_function(uumain, &[]));
        }
    });
}

/// Get the path to the false binary
fn get_false_binary() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("release");
    target_dir.join("false")
}

/// Benchmark actual process startup: false with no arguments
#[divan::bench]
fn false_spawn_no_args(bencher: Bencher) {
    let binary = get_false_binary();
    bencher.bench(|| {
        black_box(
            Command::new(&binary)
                .output()
                .expect("failed to execute false"),
        );
    });
}

/// Benchmark actual process startup: false with arbitrary arguments
#[divan::bench]
fn false_spawn_with_args(bencher: Bencher) {
    let binary = get_false_binary();
    bencher.bench(|| {
        black_box(
            Command::new(&binary)
                .args(["some", "dummy", "args"])
                .output()
                .expect("failed to execute false with args"),
        );
    });
}

fn main() {
    divan::main();
}
