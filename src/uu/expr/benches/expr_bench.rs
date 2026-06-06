// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_expr::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark `expr index` worst case: the needle char set never matches, so the
/// whole string is scanned. This is the input that exercised the old O(N * M)
/// behavior.
#[divan::bench]
fn index_no_match(bencher: Bencher) {
    let left = "A".repeat(100_000);
    let right = "B".repeat(100_000);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["index", &left, &right]));
    });
}

/// Benchmark `expr index` with a match near the end of the string.
#[divan::bench]
fn index_match_at_end(bencher: Bencher) {
    let mut left = "A".repeat(100_000);
    left.push('Z');
    let right = "Z".repeat(100_000);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["index", &left, &right]));
    });
}

fn main() {
    divan::main();
}
