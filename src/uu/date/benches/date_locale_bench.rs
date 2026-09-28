// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_date::uumain;
use uucore::benchmark::{get_bench_args, setup_test_file};

#[divan::bench]
fn file_localized_names(bencher: Bencher) {
    let path = setup_test_file("2023-05-10 12:00:00\n".repeat(1_000).as_bytes());

    bencher
        .with_inputs(|| get_bench_args(&[&"-f", &path, &"+%A %a %B %b"]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    unsafe {
        std::env::set_var("LC_ALL", "fr_FR.UTF-8");
    }
    divan::main();
}
