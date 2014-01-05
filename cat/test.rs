use std::{run, str};

#[test]
fn test_output_multi_files_print_all_chars() {
    let prog = run::process_output("build/cat",
                                   [~"cat/fixtures/alpha.txt", ~"cat/fixtures/256.txt",
                                    ~"-A", ~"-n"]).unwrap();
    let out = str::from_utf8_owned(prog.output);
    assert_eq!(out,
               ~"     1\tabcde$\n     2\tfghij$\n     3\tklmno$\n     4\tpqrst$\n     5\tuvwxyz$\n     6\t^@^A^B^C^D^E^F^G^H^I$\n     7\t^K^L^M^N^O^P^Q^R^S^T^U^V^W^X^Y^Z^[^\\^]^^^_ !\"#$%&\'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~^?M-^@M-^AM-^BM-^CM-^DM-^EM-^FM-^GM-^HM-^IM-^JM-^KM-^LM-^MM-^NM-^OM-^PM-^QM-^RM-^SM-^TM-^UM-^VM-^WM-^XM-^YM-^ZM-^[M-^\\M-^]M-^^M-^_M- M-!M-\"M-#M-$M-%M-&M-\'M-(M-)M-*M-+M-,M--M-.M-/M-0M-1M-2M-3M-4M-5M-6M-7M-8M-9M-:M-;M-<M-=M->M-?M-@M-AM-BM-CM-DM-EM-FM-GM-HM-IM-JM-KM-LM-MM-NM-OM-PM-QM-RM-SM-TM-UM-VM-WM-XM-YM-ZM-[M-\\M-]M-^M-_M-`M-aM-bM-cM-dM-eM-fM-gM-hM-iM-jM-kM-lM-mM-nM-oM-pM-qM-rM-sM-tM-uM-vM-wM-xM-yM-zM-{M-|M-}M-~M-^?");
}

#[test]
fn test_stdin_squeeze() {
    let mut prog = run::Process::new("build/cat", [~"-A"], run::ProcessOptions::new()).unwrap();

    prog.input().write(bytes!("\x00\x01\x02"));
    prog.close_input();

    let out = str::from_utf8_owned(prog.finish_with_output().output);
    assert_eq!(out, ~"^@^A^B");
}

#[test]
fn test_stdin_number_non_blank() {
    let mut prog = run::Process::new("build/cat", [~"-b", ~"-"], run::ProcessOptions::new()).unwrap();

    prog.input().write(bytes!("\na\nb\n\n\nc"));
    prog.close_input();

    let out = str::from_utf8_owned(prog.finish_with_output().output);
    assert_eq!(out, ~"\n     1\ta\n     2\tb\n\n\n     3\tc");
}
