// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_true::uumain;
use uucore::benchmark::get_bench_args;

#[divan::bench]
fn true_single_call(bencher: Bencher) {
    bencher
        .with_inputs(|| get_bench_args(&[]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
