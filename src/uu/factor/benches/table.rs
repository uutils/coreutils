use array_init::array_init;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use uu_factor::{table::*, Factors};

fn table(c: &mut Criterion) {
    let inputs = {
        // Deterministic RNG; use an explicitely-named RNG to guarantee stability
        use rand::{RngCore, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        const SEED: u64 = 0xdead_bebe_ea75_cafe;
        let mut rng = ChaCha8Rng::seed_from_u64(SEED);

        std::iter::repeat_with(move || array_init(|_| rng.next_u64()))
    };

    let mut group = c.benchmark_group("table");
    for a in inputs.take(10) {
        let a_str = format!("{:?}", a);
        group.bench_with_input(
            BenchmarkId::from_parameter("chunked_".to_owned() + &a_str),
            &a,
            |b, &a| {
                b.iter(|| factor_chunk(&mut a.clone(), &mut array_init(|_| Factors::one())));
            },
        );
        group.bench_with_input(
            BenchmarkId::from_parameter("seq_".to_owned() + &a_str),
            &a,
            |b, &a| {
                b.iter(|| {
                    let mut n_s = a.clone();
                    let mut f_s: [_; CHUNK_SIZE] = array_init(|_| Factors::one());
                    for (n, f) in n_s.iter_mut().zip(f_s.iter_mut()) {
                        factor(n, f)
                    }
                })
            },
        );
    }
    group.finish()
}

criterion_group!(benches, table);
criterion_main!(benches);
