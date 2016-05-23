use common::util::*;

static UTIL_NAME: &'static str = "pathchk";

#[test]
fn test_default_mode() {
    // test the default mode
    {
        // accept some reasonable default
        let (_, mut ucmd) = testing(UTIL_NAME);
        let result = ucmd.args(&["abc/def"]).run();
        assert_eq!(result.stdout, "");
        assert!(result.success);
    }
    {
        // fail on long inputs
        let (_, mut ucmd) = testing(UTIL_NAME);
        let result = ucmd.args(&[repeat_str("test", 20000)]).run();
        assert_eq!(result.stdout, "");
        assert!(!result.success);
    }
}
