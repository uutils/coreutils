use divan::{Bencher, black_box};
use uu_cat::uumain;
use uucore::benchmark::{run_util_function, setup_test_file};

#[divan::bench(args = [10_000, 10_000_000])]
fn cat_default(bencher: Bencher, size_bytes: usize) {
    let data = vec![b'a'; size_bytes];

    let file_path = setup_test_file(&data);
    let path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[path_str]));
    });
}

#[cfg(target_os = "linux")]
#[divan::bench(args = [10_000])]
fn cat_splice_pregenerated_pipe(bencher: Bencher, size_bytes: usize) {
    let data = vec![b'a'; size_bytes];
    let stdin_bak = rustix::io::dup(rustix::stdio::stdin()).unwrap();

    bencher
        .with_inputs(|| {
            let (pipe_r, pipe_w) = rustix::pipe::pipe().unwrap();
            // fcntl here if you want to bench 64 KiB ~ 1 MiB
            let mut file = std::fs::File::from(pipe_r);
            file.write_all(&data).unwrap();
            drop(file);
            pipe_r
        })
        .bench_local(|pipe_r| {
            use rustix::stdio::dup2_stdin;
            dup2_stdin(&pipe_r).unwrap();
            black_box(run_util_function(uumain, &["cat"]));
            dup2_stdin(&stdin_bak).unwrap();
        });
}

fn main() {
    divan::main();
}
