use common::util::*;

static UTIL_NAME: &'static str = "tsort";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_sort_call_graph() {
    new_ucmd()
        .arg("call_graph.txt")
        .run()
        .stdout_is_fixture("call_graph.expected");
}
