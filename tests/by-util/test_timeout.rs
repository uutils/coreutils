use crate::common::util::*;

// FIXME: this depends on the system having true and false in PATH
//        the best solution is probably to generate some test binaries that we can call for any
//        utility that requires executing another program (kill, for instance)
#[test]
fn test_subcommand_return_code() {
    new_ucmd!().arg("1").arg("true").succeeds();

    new_ucmd!().arg("1").arg("false").run().status_code(1);
}

#[test]
fn test_command_with_args() {
    new_ucmd!()
        .args(&["1700", "echo", "-n", "abcd"])
        .succeeds()
        .stdout_only("abcd");
}

#[test]
fn test_verbose() {
    for &verbose_flag in &["-v", "--verbose"] {
        new_ucmd!()
            .args(&[verbose_flag, ".1", "sleep", "10"])
            .fails()
            .stderr_only("timeout: sending signal TERM to command 'sleep'");
        new_ucmd!()
            .args(&[verbose_flag, "-s0", "-k.1", ".1", "sleep", "10"])
            .fails()
            .stderr_only("timeout: sending signal EXIT to command 'sleep'\ntimeout: sending signal KILL to command 'sleep'");
    }
}
