// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_echo::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark multiple consecutive invocations
#[divan::bench]
fn echo_consecutive_calls(bencher: Bencher) {
    bencher.bench(|| {
        for _ in 0..100 {
            black_box(run_util_function(uumain, &["Hello World"]));
        }
    });
}

fn main() {
    divan::main();
}
