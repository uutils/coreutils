// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
use divan::{Bencher, black_box};
#[cfg(unix)]
use uu_timeout::uumain;
#[cfg(unix)]
use uucore::benchmark::run_util_function;

/// Benchmark the fast path where the command exits immediately.
#[cfg(unix)]
#[divan::bench]
fn timeout_quick_exit(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["0.02", "true"]));
    });
}

/// Benchmark a command that runs longer than the threshold and receives the default signal.
#[cfg(unix)]
#[divan::bench]
fn timeout_enforced(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["0.02", "sleep", "0.2"]));
    });
}

fn main() {
    #[cfg(unix)]
    divan::main();
}
