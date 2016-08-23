#[macro_export]
macro_rules! assert_empty_stderr(
    ($cond:expr) => (
        if $cond.stderr.len() > 0 {
            panic!(format!("stderr: {}", $cond.stderr))
        }
    );
);

#[macro_export]
macro_rules! assert_empty_stdout(
    ($cond:expr) => (
        if $cond.stdout.len() > 0 {
            panic!(format!("stdout: {}", $cond.stdout))
        }
    );
);

#[macro_export]
macro_rules! assert_no_error(
    ($cond:expr) => (
        assert!($cond.success);
        if $cond.stderr.len() > 0 {
            panic!(format!("stderr: {}", $cond.stderr))
        }
    );
);

#[macro_export]
macro_rules! path_concat {
    ($e:expr, ..$n:expr) => {{
        use std::path::PathBuf;
        let n = $n;
        let mut pb = PathBuf::new();
        for _ in 0..n {
            pb.push($e);
        }
        pb.to_str().unwrap().to_owned()
    }};
    ($($e:expr),*) => {{
        use std::path::PathBuf;
        let mut pb = PathBuf::new();
        $(
            pb.push($e);
        )*
        pb.to_str().unwrap().to_owned()
    }};
}

#[macro_export]
macro_rules! utility_test {
    () => (
        fn util_name<'a>() -> &'a str {
            module_path!().split("_").nth(1).expect("no test name")
        }
        #[allow(dead_code)]
        fn at_and_ucmd() -> (AtPath, UCommand) {
            let ts = TestScenario::new(util_name());
            let ucmd = ts.ucmd();
            (ts.fixtures, ucmd)
        }
        #[allow(dead_code)]
        fn new_ucmd() -> UCommand {
            TestScenario::new(util_name()).ucmd()
        }
    );
    ($subcommand: expr) => (
        fn util_name<'a>() -> &'a str {
            $subcommand
        }
        #[allow(dead_code)]
        fn at_and_ucmd() -> (AtPath, UCommand) {
            let ts = TestScenario::new(util_name());
            let ucmd = ts.ucmd();
            (ts.fixtures, ucmd)
        }
        #[allow(dead_code)]
        fn new_ucmd() -> UCommand {
            TestScenario::new(util_name()).ucmd()
        }
    );
}
