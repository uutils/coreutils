use common::util::*;

static UTIL_NAME: &'static str = "who";

#[cfg(target_os = "linux")]
#[test]
fn test_count() {
    for opt in ["-q", "--count"].into_iter() {
        let scene = TestScenario::new(UTIL_NAME);
        let args = [*opt];
        scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_boot() {
    for opt in ["-b", "--boot"].into_iter() {
        let scene = TestScenario::new(UTIL_NAME);
        let args = [*opt];
        scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_heading() {
    for opt in ["-H"].into_iter() {
        let scene = TestScenario::new(UTIL_NAME);
        let args = [*opt];
        scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_short() {
    for opt in ["-s", "--short"].into_iter() {
        let scene = TestScenario::new(UTIL_NAME);
        let args = [*opt];
        scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_login() {
    for opt in ["-l", "--login"].into_iter() {
        let scene = TestScenario::new(UTIL_NAME);
        let args = [*opt];
        scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_m() {
    for opt in ["-m"].into_iter() {
        let scene = TestScenario::new(UTIL_NAME);
        let args = [*opt];
        scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_dead() {
    for opt in ["-d", "--dead"].into_iter() {
        let scene = TestScenario::new(UTIL_NAME);
        let args = [*opt];
        scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_all() {
    for opt in ["-a", "--all"].into_iter() {
        let scene = TestScenario::new(UTIL_NAME);
        let args = [*opt];
        scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
    }
}

#[cfg(target_os = "linux")]
fn expected_result(args: &[&str]) -> String {
    TestScenario::new(UTIL_NAME).cmd_keepenv(UTIL_NAME).args(args).run().stdout
}
