use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use uu_factor::numeric;

fn gcd(c: &mut Criterion) {
    let inputs = {
        // Deterministic RNG; use an explicitly-named RNG to guarantee stability
        use rand::{RngCore, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        const SEED: u64 = 0xa_b4d_1dea_dead_cafe;
        let mut rng = ChaCha8Rng::seed_from_u64(SEED);

        std::iter::repeat_with(move || (rng.next_u64(), rng.next_u64()))
    };

    let mut group = c.benchmark_group("gcd");
    for (n, m) in inputs.take(10) {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_{}", n, m)),
            &(n, m),
            |b, &(n, m)| {
                b.iter(|| numeric::gcd(n, m));
            },
        );
    }
    group.finish()
}

criterion_group!(benches, gcd);
criterion_main!(benches);
