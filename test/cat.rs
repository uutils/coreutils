use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::str;

static PROGNAME: &'static str = "./cat";

#[test]
fn test_output_multi_files_print_all_chars() {
    let po = match Command::new(PROGNAME)
                       .arg("alpha.txt")
                       .arg("256.txt")
                       .arg("-A")
                       .arg("-n")
                       .output() {

        Ok(p) => p,
        Err(err) => panic!("{}", err),
    };

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out,
               "     1\tabcde$\n     2\tfghij$\n     3\tklmno$\n     4\tpqrst$\n     5\tuvwxyz$\n     6\t^@^A^B^C^D^E^F^G^H^I$\n     7\t^K^L^M^N^O^P^Q^R^S^T^U^V^W^X^Y^Z^[^\\^]^^^_ !\"#$%&\'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~^?M-^@M-^AM-^BM-^CM-^DM-^EM-^FM-^GM-^HM-^IM-^JM-^KM-^LM-^MM-^NM-^OM-^PM-^QM-^RM-^SM-^TM-^UM-^VM-^WM-^XM-^YM-^ZM-^[M-^\\M-^]M-^^M-^_M- M-!M-\"M-#M-$M-%M-&M-\'M-(M-)M-*M-+M-,M--M-.M-/M-0M-1M-2M-3M-4M-5M-6M-7M-8M-9M-:M-;M-<M-=M->M-?M-@M-AM-BM-CM-DM-EM-FM-GM-HM-IM-JM-KM-LM-MM-NM-OM-PM-QM-RM-SM-TM-UM-VM-WM-XM-YM-ZM-[M-\\M-]M-^M-_M-`M-aM-bM-cM-dM-eM-fM-gM-hM-iM-jM-kM-lM-mM-nM-oM-pM-qM-rM-sM-tM-uM-vM-wM-xM-yM-zM-{M-|M-}M-~M-^?");
}

#[test]
fn test_stdin_squeeze() {
    let mut process = Command::new(PROGNAME)
                          .arg("-A")
                          .stdin(Stdio::piped())
                          .stdout(Stdio::piped())
                          .spawn()
                          .unwrap_or_else(|e| panic!("{}", e));

    process.stdin
           .take()
           .unwrap_or_else(|| panic!("Could not grab child process stdin"))
           .write_all("\x00\x01\x02".as_bytes())
           .unwrap_or_else(|e| panic!("{}", e));

    let po = process.wait_with_output().unwrap_or_else(|e| panic!("{}", e));
    let out = str::from_utf8(&po.stdout[..]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(out, "^@^A^B");
}

#[test]
fn test_stdin_number_non_blank() {
    let mut process = Command::new(PROGNAME)
                          .arg("-b")
                          .arg("-")
                          .stdin(Stdio::piped())
                          .stdout(Stdio::piped())
                          .spawn()
                          .unwrap_or_else(|e| panic!("{}", e));

    process.stdin
           .take()
           .unwrap_or_else(|| panic!("Could not grab child process stdin"))
           .write_all("\na\nb\n\n\nc".as_bytes())
           .unwrap_or_else(|e| panic!("{}", e));

    let po = process.wait_with_output().unwrap_or_else(|e| panic!("{}", e));
    let out = str::from_utf8(&po.stdout[..]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(out, "\n     1\ta\n     2\tb\n\n\n     3\tc");
}
