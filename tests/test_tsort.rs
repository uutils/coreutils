use common::util::*;


#[test]
fn test_sort_call_graph() {
    new_ucmd!()
        .arg("call_graph.txt")
        .run()
        .stdout_is_fixture("call_graph.expected");
}
