use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./tsort";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_sort_call_graph() {
    let input = "call_graph.txt";
    let output = "call_graph.expected";

    let po = Command::new(PROGNAME)
        .arg(input)
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    assert_eq!(String::from_utf8(po.stdout).unwrap(), String::from_utf8(get_file_contents(output).into_bytes()).unwrap());
}
