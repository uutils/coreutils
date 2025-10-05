// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::Bencher;
use uu_uniq::uumain;
use uucore::benchmark::{
    bench_util, generate_case_variation_data, generate_duplicate_heavy_data, setup_test_file,
};

/// Benchmark 1: Heavy duplicates - the main optimization target
/// Many consecutive duplicate lines that stress the line comparison optimization
#[divan::bench(args = [10_000])]
fn uniq_heavy_duplicates(bencher: Bencher, num_lines: usize) {
    // Create 1000 groups with ~10,000 duplicates each
    // This maximizes the benefit of PR #8703's optimization
    let num_groups = 1000;
    let duplicates_per_group = num_lines / num_groups;
    let data = generate_duplicate_heavy_data(num_groups, duplicates_per_group);
    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        divan::black_box(uucore::benchmark::run_util_function(
            uumain,
            &[file_path_str],
        ));
    });
}

/// Benchmark 2: Mixed duplicates with counting
/// Tests the -c flag with a mix of duplicate groups
#[divan::bench(args = [10_000])]
fn uniq_with_count(bencher: Bencher, num_lines: usize) {
    // Create more groups with fewer duplicates for varied counting
    let num_groups = num_lines / 100;
    let data = generate_duplicate_heavy_data(num_groups, 100);
    bench_util(bencher, data, &["-c"], uumain);
}

/// Benchmark 3: Case-insensitive comparison with duplicates
/// Tests the -i flag which requires case folding during comparison
#[divan::bench(args = [10_000])]
fn uniq_case_insensitive(bencher: Bencher, num_lines: usize) {
    let data = generate_case_variation_data(num_lines);
    bench_util(bencher, data, &["-i"], uumain);
}

fn main() {
    divan::main();
}
