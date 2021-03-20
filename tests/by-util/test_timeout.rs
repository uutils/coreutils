use crate::common::util::*;

// FIXME: this depends on the system having true and false in PATH
//        the best solution is probably to generate some test binaries that we can call for any
//        utility that requires executing another program (kill, for instance)
#[test]
fn test_subcommand_retcode() {
    new_ucmd!().arg("1").arg("true").succeeds();

    new_ucmd!().arg("1").arg("false").run().status_code(1);
}
