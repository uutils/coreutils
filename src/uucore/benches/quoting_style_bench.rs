// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::Bencher;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use uucore::i18n::UEncoding;
use uucore::quoting_style::{QuotingStyle, escape_bytes};

// Explicit seed to have reproducible runs.
const SEED: u64 = 0xdead_beef;

fn bench_qs_common(bencher: Bencher, style: &str, size: usize) {
    let style = QuotingStyle::parse(style).unwrap();
    let rng = SmallRng::seed_from_u64(SEED);
    let input = Vec::from_iter(rng.random_iter().take(size));

    bencher.bench_local(|| {
        std::hint::black_box(escape_bytes(&input, style, UEncoding::Ascii));
    });
}

#[divan::bench(args = [10_000, 100_000, 500_000])]
fn bench_quoting_style_c(bencher: Bencher, size: usize) {
    bench_qs_common(bencher, "c", size);
}

#[divan::bench(args = [10_000, 100_000, 500_000])]
fn bench_quoting_style_literal(bencher: Bencher, size: usize) {
    bench_qs_common(bencher, "literal", size);
}

#[divan::bench(args = [10_000, 100_000, 500_000])]
fn bench_quoting_style_shell_escape(bencher: Bencher, size: usize) {
    bench_qs_common(bencher, "shell-escape", size);
}

fn main() {
    divan::main();
}
