#[cfg(unix)]
extern crate unix_socket;
extern crate tempdir;

use common::util::*;

#[test]
fn test_output_multi_files_print_all_chars() {
    new_ucmd!()
        .args(&["alpha.txt", "256.txt", "-A", "-n"])
        .succeeds()
        .stdout_only("     1\tabcde$\n     2\tfghij$\n     3\tklmno$\n     4\tpqrst$\n     \
                5\tuvwxyz$\n     6\t^@^A^B^C^D^E^F^G^H^I$\n     \
                7\t^K^L^M^N^O^P^Q^R^S^T^U^V^W^X^Y^Z^[^\\^]^^^_ \
                !\"#$%&\'()*+,-./0123456789:;\
                <=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~^?M-^@M-^AM-^\
                BM-^CM-^DM-^EM-^FM-^GM-^HM-^IM-^JM-^KM-^LM-^MM-^NM-^OM-^PM-^QM-^RM-^SM-^TM-^UM-^V\
                M-^WM-^XM-^YM-^ZM-^[M-^\\M-^]M-^^M-^_M- \
                M-!M-\"M-#M-$M-%M-&M-\'M-(M-)M-*M-+M-,M--M-.M-/M-0M-1M-2M-3M-4M-5M-6M-7M-8M-9M-:\
                M-;M-<M-=M->M-?M-@M-AM-BM-CM-DM-EM-FM-GM-HM-IM-JM-KM-LM-MM-NM-OM-PM-QM-RM-SM-TM-U\
                M-VM-WM-XM-YM-ZM-[M-\\M-]M-^M-_M-`M-aM-bM-cM-dM-eM-fM-gM-hM-iM-jM-kM-lM-mM-nM-oM-\
                pM-qM-rM-sM-tM-uM-vM-wM-xM-yM-zM-{M-|M-}M-~M-^?");
}

#[test]
fn test_numbered_lines_no_trailing_newline() {
    new_ucmd!()
        .args(&["nonewline.txt", "alpha.txt", "-n"])
        .succeeds()
        .stdout_only("     1\ttext without a trailing newlineabcde\n     2\tfghij\n     \
                3\tklmno\n     4\tpqrst\n     5\tuvwxyz\n");
}

#[test]
fn test_stdin_show_nonprinting() {
    for same_param in vec!["-v", "--show-nonprinting"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("\t^@");
    }
}

#[test]
fn test_stdin_show_tabs() {
    for same_param in vec!["-T", "--show-tabs"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("^I\0");
    }
}


#[test]
fn test_stdin_show_ends() {
    for same_param in vec!["-E", "--show-ends"] {
        new_ucmd!()
            .args(&[same_param,"-"])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("\t\0$");
    }
}

#[test]
fn test_stdin_show_all() {
    for same_param in vec!["-A", "--show-all"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("^I^@$");
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
#[cfg(foo)]
fn test_domain_socket() {
    use std::thread;
    use self::unix_socket::UnixListener;
    use self::tempdir::TempDir;
    use std::io::prelude::*;

    let dir = TempDir::new("unix_socket").expect("failed to create dir");
    let socket_path = dir.path().join("sock");
    let listener = UnixListener::bind(&socket_path).expect("failed to create socket");

    let thread = thread::spawn(move || {
        let mut stream = listener.accept().expect("failed to accept connection").0;
        stream.write_all(b"a\tb").expect("failed to write test data");
    });

    new_ucmd!()
        .args(&[socket_path])
        .succeeds()
        .stdout_only("a\tb");

    thread.join().unwrap();
}
