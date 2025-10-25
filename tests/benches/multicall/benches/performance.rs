// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::process::Command;

fn multicall(c: &mut Criterion) {
    let utilities = ["cat", "echo", "pwd", "true", "false"];

    let mut group = c.benchmark_group("multicall");

    // Benchmark startup time
    group.bench_function("startup", |b| {
        b.iter(|| {
            let output = Command::new("target/release/coreutils")
                .arg("--version")
                .output()
                .expect("Failed to execute coreutils");
            black_box(output);
        });
    });

    // Benchmark utility execution
    for utility in &utilities {
        group.bench_function(*utility, |b| {
            b.iter(|| {
                let output = Command::new("target/release/coreutils")
                    .args(&[utility, "--version"])
                    .output()
                    .expect("Failed to execute utility");
                black_box(output);
            });
        });
    }

    // Benchmark binary size
    group.bench_function("binary_size", |b| {
        b.iter(|| {
            let metadata = std::fs::metadata("target/release/coreutils")
                .expect("Failed to get binary metadata");
            let size_bytes = metadata.len();
            black_box(size_bytes);
        });
    });

    group.finish();
}

criterion_group!(benches, multicall);
criterion_main!(benches);
