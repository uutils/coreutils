// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore aeiou

//! Benchmarks for `tr`.
//!
//! `tr` only reads stdin, so each bench redirects fd 0 onto a prepared
//! input file before invoking `uumain`. fd 1 is redirected to /dev/null
//! so the translated output does not flood the harness's terminal.
//! Both fds are restored after each benchmark.

#[cfg(unix)]
mod benches {
    use divan::{Bencher, black_box};
    use uu_tr::uumain;
    use uucore::benchmark::{get_bench_args, setup_test_file, text_data};

    fn bench_tr_with_stdin(bencher: Bencher, data: &[u8], args: Vec<std::ffi::OsString>) {
        let file_path = setup_test_file(data);
        let file = std::fs::File::open(file_path).unwrap();
        let devnull = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .unwrap();
        let stdin_bak = rustix::io::dup(rustix::stdio::stdin()).unwrap();
        let stdout_bak = rustix::io::dup(rustix::stdio::stdout()).unwrap();
        rustix::stdio::dup2_stdin(&file).unwrap();
        rustix::stdio::dup2_stdout(&devnull).unwrap();

        bencher
            .with_inputs(|| {
                rustix::fs::seek(&file, rustix::fs::SeekFrom::Start(0)).unwrap();
                args.clone().into_iter()
            })
            .bench_local_values(|args| black_box(uumain(args)));

        rustix::stdio::dup2_stdin(&stdin_bak).unwrap();
        rustix::stdio::dup2_stdout(&stdout_bak).unwrap();
    }

    const SIZE_MB: usize = 16;

    /// ASCII lowercase->uppercase range translation.
    /// Exercises the AVX2 ASCII-range fast path on x86_64 hosts that
    /// support it, and the scalar range fallback on other targets.
    #[divan::bench]
    fn tr_ascii_range_lower_to_upper(bencher: Bencher) {
        let data = text_data::generate_by_size(SIZE_MB, 80);
        bench_tr_with_stdin(bencher, &data, get_bench_args(&[&"a-z", &"A-Z"]));
    }

    /// Single-character replacement. Exercises the existing
    /// `process_single_char_replace` SIMD path; guards against
    /// regressions outside the new range fast path.
    #[divan::bench]
    fn tr_single_char_replace(bencher: Bencher) {
        let data = text_data::generate_by_size(SIZE_MB, 80);
        bench_tr_with_stdin(bencher, &data, get_bench_args(&[&"a", &"b"]));
    }

    /// Multi-character set translation. Falls through to the
    /// 256-byte translation table path (no fast path applies).
    #[divan::bench]
    fn tr_multi_char_translate(bencher: Bencher) {
        let data = text_data::generate_by_size(SIZE_MB, 80);
        bench_tr_with_stdin(bencher, &data, get_bench_args(&[&"aeiou", &"AEIOU"]));
    }

    /// Delete an ASCII range — covers the deletion path.
    #[divan::bench]
    fn tr_delete_ascii_range(bencher: Bencher) {
        let data = text_data::generate_by_size(SIZE_MB, 80);
        bench_tr_with_stdin(bencher, &data, get_bench_args(&[&"-d", &"a-z"]));
    }

    /// Delete a single character (the newlines).
    #[divan::bench]
    fn tr_delete_single_char(bencher: Bencher) {
        let data = text_data::generate_by_size(SIZE_MB, 80);
        bench_tr_with_stdin(bencher, &data, get_bench_args(&[&"-d", &"\\n"]));
    }
}

fn main() {
    // Rewind happens before each sample; force one iteration after CLI/env overrides.
    #[cfg(not(codspeed))]
    divan::Divan::from_args().sample_size(1).main();
    #[cfg(codspeed)]
    divan::main();
}
