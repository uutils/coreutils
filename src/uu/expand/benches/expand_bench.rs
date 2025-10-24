// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::Bencher;
use uu_expand::uumain;
use uucore::benchmark::{bench_util, generate_multi_tab_text, generate_tabbed_text};

/// Benchmark expanding tabs on files with many short lines
#[divan::bench(args = [100_000])]
fn expand_many_lines(bencher: Bencher, num_lines: usize) {
    let data = generate_tabbed_text(num_lines);
    bench_util(bencher, data, &[], uumain);
}

/// Benchmark expanding tabs with custom tab stops
#[divan::bench(args = [50_000])]
fn expand_custom_tabstops(bencher: Bencher, num_lines: usize) {
    let data = generate_multi_tab_text(num_lines);
    bench_util(bencher, data, &["--tabs=4,8,12"], uumain);
}

fn main() {
    divan::main();
}
