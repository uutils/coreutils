use crate::common::util::*;

fn vec_of_size(n: usize) -> Vec<u8> {
    let mut result = Vec::new();
    for _ in 0..n {
        result.push('a' as u8);
    }
    assert_eq!(result.len(), n);
    result
}

#[test]
fn test_output_simple() {
    new_ucmd!()
        .args(&["alpha.txt"])
        .succeeds()
        .stdout_only("abcde\nfghij\nklmno\npqrst\nuvwxyz\n");
}

#[test]
fn test_no_options() {
    for fixture in &["empty.txt", "alpha.txt", "nonewline.txt"] {
        // Give fixture through command line file argument
        new_ucmd!()
            .args(&[fixture])
            .succeeds()
            .stdout_is_fixture(fixture);
        // Give fixture through stdin
        new_ucmd!()
            .pipe_in_fixture(fixture)
            .succeeds()
            .stdout_is_fixture(fixture);
    }
}

#[test]
fn test_no_options_big_input() {
    for n in &[
        0,
        1,
        42,
        16 * 1024 - 7,
        16 * 1024 - 1,
        16 * 1024,
        16 * 1024 + 1,
        16 * 1024 + 3,
        32 * 1024,
        64 * 1024,
        80 * 1024,
        96 * 1024,
        112 * 1024,
        128 * 1024,
    ] {
        let data = vec_of_size(*n);
        let data2 = data.clone();
        assert_eq!(data.len(), data2.len());
        new_ucmd!().pipe_in(data).succeeds().stdout_is_bytes(&data2);
    }
}

#[test]
#[cfg(unix)]
fn test_fifo_symlink() {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::thread;

    let s = TestScenario::new(util_name!());
    s.fixtures.mkdir("dir");
    s.fixtures.mkfifo("dir/pipe");
    assert!(s.fixtures.is_fifo("dir/pipe"));

    // Make cat read the pipe through a symlink
    s.fixtures.symlink_file("dir/pipe", "sympipe");
    let proc = s.ucmd().args(&["sympipe"]).run_no_wait();

    let data = vec_of_size(128 * 1024);
    let data2 = data.clone();

    let pipe_path = s.fixtures.plus("dir/pipe");
    let thread = thread::spawn(move || {
        let mut pipe = OpenOptions::new()
            .write(true)
            .create(false)
            .open(pipe_path)
            .unwrap();
        pipe.write_all(&data).unwrap();
    });

    let output = proc.wait_with_output().unwrap();
    assert_eq!(&output.stdout, &data2);
    thread.join().unwrap();
}

#[test]
fn test_output_multi_files_print_all_chars() {
    new_ucmd!()
        .args(&["alpha.txt", "256.txt", "-A", "-n"])
        .succeeds()
        .stdout_only(
            "     1\tabcde$\n     2\tfghij$\n     3\tklmno$\n     4\tpqrst$\n     \
             5\tuvwxyz$\n     6\t^@^A^B^C^D^E^F^G^H^I$\n     \
             7\t^K^L^M^N^O^P^Q^R^S^T^U^V^W^X^Y^Z^[^\\^]^^^_ \
             !\"#$%&\'()*+,-./0123456789:;\
             <=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~^?M-^@M-^AM-^\
             BM-^CM-^DM-^EM-^FM-^GM-^HM-^IM-^JM-^KM-^LM-^MM-^NM-^OM-^PM-^QM-^RM-^SM-^TM-^UM-^V\
             M-^WM-^XM-^YM-^ZM-^[M-^\\M-^]M-^^M-^_M- \
             M-!M-\"M-#M-$M-%M-&M-\'M-(M-)M-*M-+M-,M--M-.M-/M-0M-1M-2M-3M-4M-5M-6M-7M-8M-9M-:\
             M-;M-<M-=M->M-?M-@M-AM-BM-CM-DM-EM-FM-GM-HM-IM-JM-KM-LM-MM-NM-OM-PM-QM-RM-SM-TM-U\
             M-VM-WM-XM-YM-ZM-[M-\\M-]M-^M-_M-`M-aM-bM-cM-dM-eM-fM-gM-hM-iM-jM-kM-lM-mM-nM-oM-\
             pM-qM-rM-sM-tM-uM-vM-wM-xM-yM-zM-{M-|M-}M-~M-^?",
        );
}

#[test]
fn test_numbered_lines_no_trailing_newline() {
    new_ucmd!()
        .args(&["nonewline.txt", "alpha.txt", "-n"])
        .succeeds()
        .stdout_only(
            "     1\ttext without a trailing newlineabcde\n     2\tfghij\n     \
             3\tklmno\n     4\tpqrst\n     5\tuvwxyz\n",
        );
}

#[test]
fn test_stdin_show_nonprinting() {
    for same_param in vec!["-v", "--show-nonprinting"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("\t^@\n");
    }
}

#[test]
fn test_stdin_show_tabs() {
    for same_param in vec!["-T", "--show-tabs"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("^I\0\n");
    }
}

#[test]
fn test_stdin_show_ends() {
    for same_param in vec!["-E", "--show-ends"] {
        new_ucmd!()
            .args(&[same_param, "-"])
            .pipe_in("\t\0\n\t")
            .succeeds()
            .stdout_only("\t\0$\n\t");
    }
}

#[test]
fn test_stdin_show_all() {
    for same_param in vec!["-A", "--show-all"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("^I^@$\n");
    }
}

#[test]
fn test_stdin_nonprinting_and_endofline() {
    new_ucmd!()
        .args(&["-e"])
        .pipe_in("\t\0\n")
        .succeeds()
        .stdout_only("\t^@$\n");
}

#[test]
fn test_stdin_nonprinting_and_tabs() {
    new_ucmd!()
        .args(&["-t"])
        .pipe_in("\t\0\n")
        .succeeds()
        .stdout_only("^I^@\n");
}

#[test]
fn test_stdin_squeeze_blank() {
    for same_param in vec!["-s", "--squeeze-blank"] {
        new_ucmd!()
            .arg(same_param)
            .pipe_in("\n\na\n\n\n\n\nb\n\n\n")
            .succeeds()
            .stdout_only("\na\n\nb\n\n");
    }
}

#[test]
fn test_stdin_number_non_blank() {
    for same_param in vec!["-b", "--number-nonblank"] {
        new_ucmd!()
            .arg(same_param)
            .arg("-")
            .pipe_in("\na\nb\n\n\nc")
            .succeeds()
            .stdout_only("\n     1\ta\n     2\tb\n\n\n     3\tc");
    }
}

#[test]
fn test_non_blank_overrides_number() {
    for same_param in vec!["-b", "--number-nonblank"] {
        new_ucmd!()
            .args(&[same_param, "-"])
            .pipe_in("\na\nb\n\n\nc")
            .succeeds()
            .stdout_only("\n     1\ta\n     2\tb\n\n\n     3\tc");
    }
}

#[test]
fn test_squeeze_blank_before_numbering() {
    for same_param in vec!["-s", "--squeeze-blank"] {
        new_ucmd!()
            .args(&[same_param, "-n", "-"])
            .pipe_in("a\n\n\nb")
            .succeeds()
            .stdout_only("     1\ta\n     2\t\n     3\tb");
    }
}

#[test]
#[cfg(unix)]
fn test_domain_socket() {
    use std::io::prelude::*;
    use std::thread;
    use tempdir::TempDir;
    use unix_socket::UnixListener;

    let dir = TempDir::new("unix_socket").expect("failed to create dir");
    let socket_path = dir.path().join("sock");
    let listener = UnixListener::bind(&socket_path).expect("failed to create socket");

    let thread = thread::spawn(move || {
        let mut stream = listener.accept().expect("failed to accept connection").0;
        stream
            .write_all(b"a\tb")
            .expect("failed to write test data");
    });

    new_ucmd!()
        .args(&[socket_path])
        .succeeds()
        .stdout_only("a\tb");

    thread.join().unwrap();
}
