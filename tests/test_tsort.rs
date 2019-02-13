use common::util::*;


#[test]
fn test_sort_call_graph() {
    new_ucmd!()
        .arg("call_graph.txt")
        .run()
        .stdout_is_fixture("call_graph.expected");
}

#[test]
fn test_sort_self_loop() {
    new_ucmd!()
        .pipe_in("first first\nfirst second second second")
        .succeeds().stdout_only("first\nsecond\n");
}
