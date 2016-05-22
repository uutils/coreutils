use common::util::*;

static UTIL_NAME: &'static str = "tsort";

#[test]
fn test_sort_call_graph() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let input = "call_graph.txt";
    let output = "call_graph.expected";

    let out = ucmd.arg(input)
                  .run()
                  .stdout;

    assert_eq!(out,
               String::from_utf8(at.read(output).into_bytes()).unwrap());
}
