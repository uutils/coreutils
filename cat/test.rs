use std::rt::io::process::{Process, ProcessConfig, CreatePipe, Ignored};
use std::rt::io::{Reader, Writer};
use std::rt::io::pipe::PipeStream;
use std::str;

fn main() {
    test_output_multi_files_print_all_chars();
    test_stdin_squeeze();
    test_stdin_number_non_blank();
}

fn read_all(input: &mut Reader) -> ~str {
    let mut ret = ~"";
    let mut buf = [0, ..1024];
    loop {
        match input.read(buf) {
            None => { break }
            Some(n) => { ret = ret + str::from_utf8(buf.slice_to(n)); }
        }
    }
    return ret;
}

fn test_output_multi_files_print_all_chars() {
    let output = PipeStream::new().unwrap();
    let io = ~[Ignored,
               CreatePipe(output, false, true)];
    let args = ProcessConfig {
        program: "build/cat",
        args: [~"cat/fixtures/alpha.txt", ~"cat/fixtures/256.txt", ~"-A", ~"-n"],
        env: None,
        cwd: None,
        io: io,
    };
    let mut p = Process::new(args).expect("proc fail");
    let out = read_all(p.io[1].get_mut_ref() as &mut Reader);
    assert_eq!(p.wait(), 0);
    assert_eq!(out, ~"     1\tabcde$\n     2\tfghij$\n     3\tklmno$\n     4\tpqrst$\n     5\tuvwxyz$\n     6\t^@^A^B^C^D^E^F^G^H^I$\n     7\t^K^L^M^N^O^P^Q^R^S^T^U^V^W^X^Y^Z^[^\\^]^^^_ !\"#$%&\'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~^?M-^@M-^AM-^BM-^CM-^DM-^EM-^FM-^GM-^HM-^IM-^JM-^KM-^LM-^MM-^NM-^OM-^PM-^QM-^RM-^SM-^TM-^UM-^VM-^WM-^XM-^YM-^ZM-^[M-^\\M-^]M-^^M-^_M- M-!M-\"M-#M-$M-%M-&M-\'M-(M-)M-*M-+M-,M--M-.M-/M-0M-1M-2M-3M-4M-5M-6M-7M-8M-9M-:M-;M-<M-=M->M-?M-@M-AM-BM-CM-DM-EM-FM-GM-HM-IM-JM-KM-LM-MM-NM-OM-PM-QM-RM-SM-TM-UM-VM-WM-XM-YM-ZM-[M-\\M-]M-^M-_M-`M-aM-bM-cM-dM-eM-fM-gM-hM-iM-jM-kM-lM-mM-nM-oM-pM-qM-rM-sM-tM-uM-vM-wM-xM-yM-zM-{M-|M-}M-~M-^?");
}

fn test_stdin_squeeze() {
    let input = PipeStream::new().unwrap();
    let output = PipeStream::new().unwrap();
    let io = ~[CreatePipe(input, true, false),
               CreatePipe(output, false, true)];
    let args = ProcessConfig {
        program: "build/cat",
        args: [~"-A"],
        env: None,
        cwd: None,
        io: io,
    };
    let mut p = Process::new(args).expect("proc fail");
    p.io[0].get_mut_ref().write("\x00\x01\x02".as_bytes());
    p.io[0] = None; // close stdin;
    let out = read_all(p.io[1].get_mut_ref() as &mut Reader);
    assert_eq!(p.wait(), 0);
    assert_eq!(out, ~"^@^A^B");
}

fn test_stdin_number_non_blank() {
    let input = PipeStream::new().unwrap();
    let output = PipeStream::new().unwrap();
    let io = ~[CreatePipe(input, true, false),
               CreatePipe(output, false, true)];
    let args = ProcessConfig {
        program: "build/cat",
        args: [~"-b", ~"-"],
        env: None,
        cwd: None,
        io: io,
    };
    let mut p = Process::new(args).expect("proc fail");
    p.io[0].get_mut_ref().write("\na\nb\n\n\nc".as_bytes());
    p.io[0] = None; // close stdin;
    let out = read_all(p.io[1].get_mut_ref() as &mut Reader);
    assert_eq!(p.wait(), 0);
    assert_eq!(out, ~"\n     1\ta\n     2\tb\n\n\n     3\tc");
}
