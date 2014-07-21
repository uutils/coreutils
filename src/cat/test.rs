use std::io::process::Command;
use std::str;

#[test]
fn test_output_multi_files_print_all_chars() {
    let po = match Command::new("build/cat")
                                .arg("src/cat/fixtures/alpha.txt")
                                .arg("src/cat/fixtures/256.txt")
                                .arg("-A")
                                .arg("-n").output() {

        Ok(p) => p,
        Err(err) => fail!("{}", err),
    };

    let out = str::from_utf8(po.output.as_slice()).unwrap();
    assert_eq!(out,
               "     1\tabcde$\n     2\tfghij$\n     3\tklmno$\n     4\tpqrst$\n     5\tuvwxyz$\n     6\t^@^A^B^C^D^E^F^G^H^I$\n     7\t^K^L^M^N^O^P^Q^R^S^T^U^V^W^X^Y^Z^[^\\^]^^^_ !\"#$%&\'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~^?M-^@M-^AM-^BM-^CM-^DM-^EM-^FM-^GM-^HM-^IM-^JM-^KM-^LM-^MM-^NM-^OM-^PM-^QM-^RM-^SM-^TM-^UM-^VM-^WM-^XM-^YM-^ZM-^[M-^\\M-^]M-^^M-^_M- M-!M-\"M-#M-$M-%M-&M-\'M-(M-)M-*M-+M-,M--M-.M-/M-0M-1M-2M-3M-4M-5M-6M-7M-8M-9M-:M-;M-<M-=M->M-?M-@M-AM-BM-CM-DM-EM-FM-GM-HM-IM-JM-KM-LM-MM-NM-OM-PM-QM-RM-SM-TM-UM-VM-WM-XM-YM-ZM-[M-\\M-]M-^M-_M-`M-aM-bM-cM-dM-eM-fM-gM-hM-iM-jM-kM-lM-mM-nM-oM-pM-qM-rM-sM-tM-uM-vM-wM-xM-yM-zM-{M-|M-}M-~M-^?");
}

#[test]
fn test_stdin_squeeze() {
    let mut process= Command::new("build/cat").arg("-A").spawn().unwrap();

    process.stdin.take_unwrap().write(b"\x00\x01\x02").unwrap();
    let po = process.wait_with_output().unwrap();
    let out = str::from_utf8(po.output.as_slice()).unwrap();

    assert_eq!(out, "^@^A^B");
}

#[test]
fn test_stdin_number_non_blank() {
    let mut process = Command::new("build/cat").arg("-b").arg("-").spawn().unwrap();

    process.stdin.take_unwrap().write(b"\na\nb\n\n\nc").unwrap();
    let po = process.wait_with_output().unwrap();
    let out =  str::from_utf8(po.output.as_slice()).unwrap();

    assert_eq!(out, "\n     1\ta\n     2\tb\n\n\n     3\tc");
}
