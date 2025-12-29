// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs

use divan::{Bencher, black_box};
use uu_factor::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark multiple u64 digits
#[divan::bench(args = [(2)])]
fn factor_multiple_u64s(bencher: Bencher, start_num: u64) {
    bencher
        // this is a range of 5000 different u128 integers
        .with_inputs(|| (start_num, start_num + 2500))
        .bench_values(|(start_u64, end_u64)| {
            for u64_digit in start_u64..=end_u64 {
                black_box(run_util_function(uumain, &[&u64_digit.to_string()]));
            }
        });
}

fn main() {
    divan::main();
}
