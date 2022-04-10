use array_init::array_init;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use uu_factor::{table::*, Factors};

fn table(c: &mut Criterion) {
    #[cfg(target_os = "linux")]
    check_personality();

    const INPUT_SIZE: usize = 128;
    assert!(
        INPUT_SIZE % CHUNK_SIZE == 0,
        "INPUT_SIZE ({}) is not divisible by CHUNK_SIZE ({})",
        INPUT_SIZE,
        CHUNK_SIZE
    );
    let inputs = {
        // Deterministic RNG; use an explicitly-named RNG to guarantee stability
        use rand::{RngCore, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        const SEED: u64 = 0xdead_bebe_ea75_cafe; // spell-checker:disable-line
        let mut rng = ChaCha8Rng::seed_from_u64(SEED);

        std::iter::repeat_with(move || array_init::<_, _, INPUT_SIZE>(|_| rng.next_u64()))
    };

    let mut group = c.benchmark_group("table");
    group.throughput(Throughput::Elements(INPUT_SIZE as _));
    for a in inputs.take(10) {
        let a_str = format!("{:?}", a);
        group.bench_with_input(BenchmarkId::new("factor_chunk", &a_str), &a, |b, &a| {
            b.iter(|| {
                let mut n_s = a.clone();
                let mut f_s: [_; INPUT_SIZE] = array_init(|_| Factors::one());
                for (n_s, f_s) in n_s.chunks_mut(CHUNK_SIZE).zip(f_s.chunks_mut(CHUNK_SIZE)) {
                    factor_chunk(n_s.try_into().unwrap(), f_s.try_into().unwrap())
                }
            })
        });
        group.bench_with_input(BenchmarkId::new("factor", &a_str), &a, |b, &a| {
            b.iter(|| {
                let mut n_s = a.clone();
                let mut f_s: [_; INPUT_SIZE] = array_init(|_| Factors::one());
                for (n, f) in n_s.iter_mut().zip(f_s.iter_mut()) {
                    factor(n, f)
                }
            })
        });
    }
    group.finish()
}

#[cfg(target_os = "linux")]
fn check_personality() {
    use std::fs;
    const ADDR_NO_RANDOMIZE: u64 = 0x0040000;
    const PERSONALITY_PATH: &'static str = "/proc/self/personality";

    let p_string = fs::read_to_string(PERSONALITY_PATH)
        .expect(&format!("Couldn't read '{}'", PERSONALITY_PATH))
        .strip_suffix("\n")
        .unwrap()
        .to_owned();

    let personality = u64::from_str_radix(&p_string, 16).expect(&format!(
        "Expected a hex value for personality, got '{:?}'",
        p_string
    ));
    if personality & ADDR_NO_RANDOMIZE == 0 {
        eprintln!(
            "WARNING: Benchmarking with ASLR enabled (personality is {:x}), results might not be reproducible.",
            personality
        );
    }
}

criterion_group!(benches, table);
criterion_main!(benches);
