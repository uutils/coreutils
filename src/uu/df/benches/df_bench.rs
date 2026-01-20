// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uu_df::uumain;
use uucore::benchmark::get_bench_args;

fn create_deep_directory(base_dir: &std::path::Path, depth: usize) -> PathBuf {
    let mut current = base_dir.to_path_buf();
    env::set_current_dir(&current).unwrap();

    for _ in 0..depth {
        current = current.join("d");
        fs::create_dir("d").unwrap();
        env::set_current_dir("d").unwrap();
    }
    current
}

#[divan::bench]
fn df_deep_directory(bencher: Bencher) {
    const DEPTH: usize = 20000;

    let original_dir = env::current_dir().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _deep_path = create_deep_directory(temp_dir.path(), DEPTH);

    bencher
        .with_inputs(|| get_bench_args(&[]))
        .bench_values(|args| black_box(uumain(args)));

    env::set_current_dir(original_dir).unwrap();
}

#[divan::bench]
fn df_with_path(bencher: Bencher) {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    bencher
        .with_inputs(|| get_bench_args(&[&temp_path]))
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
