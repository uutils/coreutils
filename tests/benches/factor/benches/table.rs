// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs

use array_init::array_init;
use divan::Bencher;

fn main() {
    divan::main();
}

#[divan::bench()]
fn factor_table(bencher: Bencher) {
    #[cfg(target_os = "linux")]
    check_personality();

    const INPUT_SIZE: usize = 128;

    let inputs = {
        // Deterministic RNG; use an explicitly-named RNG to guarantee stability
        use rand::{RngCore, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        const SEED: u64 = 0xdead_bebe_ea75_cafe; // spell-checker:disable-line
        let mut rng = ChaCha8Rng::seed_from_u64(SEED);

        std::iter::repeat_with(move || array_init::<_, _, INPUT_SIZE>(|_| rng.next_u64()))
            .take(10)
            .collect::<Vec<_>>()
    };

    bencher.bench(|| {
        for a in &inputs {
            for n in a {
                divan::black_box(num_prime::nt_funcs::factors(*n, None));
            }
        }
    });
}

#[cfg(target_os = "linux")]
fn check_personality() {
    use std::fs;
    const ADDR_NO_RANDOMIZE: u64 = 0x0040000;
    const PERSONALITY_PATH: &str = "/proc/self/personality";

    let p_string = fs::read_to_string(PERSONALITY_PATH)
        .unwrap_or_else(|_| panic!("Couldn't read '{PERSONALITY_PATH}'"))
        .strip_suffix('\n')
        .unwrap()
        .to_owned();

    let personality = u64::from_str_radix(&p_string, 16)
        .unwrap_or_else(|_| panic!("Expected a hex value for personality, got '{p_string:?}'"));
    if personality & ADDR_NO_RANDOMIZE == 0 {
        eprintln!(
            "WARNING: Benchmarking with ASLR enabled (personality is {personality:x}), results might not be reproducible."
        );
    }
}
