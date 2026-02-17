// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::process::Command;
use uu_true::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark the common case: true with no arguments
#[divan::bench]
fn true_no_args(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &[]));
    });
}

/// Benchmark true with arbitrary arguments (e.g., called via dummy symlink)
#[divan::bench]
fn true_with_args(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["some", "dummy", "args"]));
    });
}

/// Benchmark multiple consecutive invocations
#[divan::bench]
fn true_consecutive_calls(bencher: Bencher) {
    bencher.bench(|| {
        for _ in 0..100 {
            black_box(run_util_function(uumain, &[]));
        }
    });
}

/// Get the path to the true binary
fn get_true_binary() -> std::path::PathBuf {
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
    target_dir.join("true")
}

/// Benchmark actual process startup: true with no arguments
#[divan::bench]
fn true_spawn_no_args(bencher: Bencher) {
    let binary = get_true_binary();
    bencher.bench(|| {
        black_box(
            Command::new(&binary)
                .output()
                .expect("failed to execute true"),
        );
    });
}

/// Benchmark actual process startup: true with arbitrary arguments
#[divan::bench]
fn true_spawn_with_args(bencher: Bencher) {
    let binary = get_true_binary();
    bencher.bench(|| {
        black_box(
            Command::new(&binary)
                .args(["some", "dummy", "args"])
                .output()
                .expect("failed to execute true with args"),
        );
    });
}

fn main() {
    divan::main();
}
