// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Base16 output and Base58 conversion benchmarks.

#[cfg(unix)]
mod benches {
    use divan::{Bencher, black_box};
    use std::fs::File;
    use uu_basenc::uumain;
    use uucore::benchmark::{run_util_function, setup_test_file};

    const INPUT_SIZE: usize = 16 * 1024 * 1024;
    const ENCODED_SIZE: usize = INPUT_SIZE * 2;
    const OUTPUT_SIZE: u64 =
        (ENCODED_SIZE + ENCODED_SIZE.div_ceil(uu_base32::base_common::WRAP_DEFAULT)) as u64;

    fn input_file() -> std::path::PathBuf {
        setup_test_file(&vec![0xA5; INPUT_SIZE])
    }

    fn redirect_stdout(output: &File) -> rustix::fd::OwnedFd {
        let stdout_backup = rustix::io::dup(rustix::stdio::stdout()).unwrap();
        rustix::stdio::dup2_stdout(output).unwrap();
        stdout_backup
    }

    fn restore_stdout(stdout_backup: &rustix::fd::OwnedFd) {
        rustix::stdio::dup2_stdout(stdout_backup).unwrap();
    }

    /// Encode Base16 to `/dev/null`.
    #[divan::bench]
    fn base16_encode_to_dev_null(bencher: Bencher) {
        let input_path = input_file();
        let input = input_path.to_str().unwrap();
        let dev_null = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .unwrap();
        let stdout_backup = redirect_stdout(&dev_null);

        assert_eq!(run_util_function(uumain, &["--base16", input]), 0);

        bencher.bench_local(|| {
            black_box(run_util_function(uumain, &["--base16", input]));
        });

        restore_stdout(&stdout_backup);
    }

    /// Encode Base16 to a file.
    #[divan::bench]
    fn base16_encode_to_file(bencher: Bencher) {
        let input_path = input_file();
        let input = input_path.to_str().unwrap();
        let output = tempfile::tempfile().unwrap();
        let stdout_backup = redirect_stdout(&output);

        assert_eq!(run_util_function(uumain, &["--base16", input]), 0);
        assert_eq!(output.metadata().unwrap().len(), OUTPUT_SIZE);

        bencher
            .with_inputs(|| {
                output.set_len(0).unwrap();
                rustix::fs::seek(&output, rustix::fs::SeekFrom::Start(0)).unwrap();
            })
            .bench_local_values(|()| {
                black_box(run_util_function(uumain, &["--base16", input]));
            });

        restore_stdout(&stdout_backup);
    }

    /// Base58 treats the whole input as one big number, so its cost grows
    /// faster than linearly with the input size: keep these inputs small.
    const BASE58_ENCODE_INPUT_SIZE: usize = 24 * 1024;
    const BASE58_DECODE_INPUT_SIZE: usize = 64 * 1024;

    fn run_to_dev_null(bencher: Bencher, args: &[&str]) {
        let dev_null = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .unwrap();
        let stdout_backup = redirect_stdout(&dev_null);

        assert_eq!(run_util_function(uumain, args), 0);

        bencher.bench_local(|| {
            black_box(run_util_function(uumain, args));
        });

        restore_stdout(&stdout_backup);
    }

    /// Encode Base58 to `/dev/null`.
    #[divan::bench]
    fn base58_encode(bencher: Bencher) {
        let data: Vec<u8> = (0..BASE58_ENCODE_INPUT_SIZE)
            .map(|i| (i * 131 + 7) as u8)
            .collect();
        let input_path = setup_test_file(&data);
        run_to_dev_null(bencher, &["--base58", input_path.to_str().unwrap()]);
    }

    /// Decode Base58 to `/dev/null`.
    #[divan::bench]
    fn base58_decode(bencher: Bencher) {
        const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        // Start at index 1 so the encoded number has no leading '1' (zero bytes).
        let data: Vec<u8> = (1..=BASE58_DECODE_INPUT_SIZE)
            .map(|i| ALPHABET[(i * 37) % ALPHABET.len()])
            .collect();
        let input_path = setup_test_file(&data);
        run_to_dev_null(bencher, &["-d", "--base58", input_path.to_str().unwrap()]);
    }
}

fn main() {
    divan::main();
}
