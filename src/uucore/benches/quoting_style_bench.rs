#[cfg(feature = "quoting-style")]
mod quoting_style_bench {
    use divan::Bencher;
    use rand::{RngExt, SeedableRng, rngs::SmallRng};
    use uucore::i18n::UEncoding;
    use uucore::quoting_style::{QuotingStyle, escape_name_inner};

    #[cfg(feature = "quoting-style")]
    const SEED: u64 = 0xdead_beef;

    #[cfg(feature = "quoting-style")]
    fn bench_qs_common(bencher: Bencher, style: &str, size: usize) {
        let style = QuotingStyle::parse(style).unwrap();
        let rng = SmallRng::seed_from_u64(SEED);
        let input = Vec::from_iter(rng.random_iter().take(size));

        bencher.bench_local(|| {
            std::hint::black_box(escape_name_inner(&input, style, false, UEncoding::Ascii));
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
}

fn main() {
    divan::main();
}
