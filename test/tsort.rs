use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

static PROGNAME: &'static str = "./tsort";

fn get_file_contents(name: &str) -> Vec<u8> {
    let mut f = File::open(Path::new(name)).unwrap();
    let mut contents: Vec<u8> = vec!();
    let _ = f.read_to_end(&mut contents);
    contents
}

#[test]
fn test_sort_call_graph() {
    let input = "call_graph.txt";
    let output = "call_graph.expected";

    let po = Command::new(PROGNAME)
        .arg(input)
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    assert_eq!(String::from_utf8(po.stdout).unwrap(), String::from_utf8(get_file_contents(output)).unwrap());
}
