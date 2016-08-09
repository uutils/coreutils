use common::util::*;

static UTIL_NAME: &'static str = "pathchk";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_default_mode() {
    // test the default mode
    {
        // accept some reasonable default
        let result = new_ucmd()
            .args(&["abc/def"]).run();
        assert_eq!(result.stdout, "");
        assert!(result.success);
    }
    {
        // fail on long inputs
        let result = new_ucmd()
            .args(&[repeat_str("test", 20000)]).run();
        assert_eq!(result.stdout, "");
        assert!(!result.success);
    }
}
