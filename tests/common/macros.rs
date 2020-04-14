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
macro_rules! util_name {
    () => {
        module_path!().split("_").nth(1).expect("no test name")
    };
}

#[macro_export]
macro_rules! new_ucmd {
    () => {
        TestScenario::new(util_name!()).ucmd()
    };
}

#[macro_export]
macro_rules! at_and_ucmd {
    () => {{
        let ts = TestScenario::new(util_name!());
        (ts.fixtures.clone(), ts.ucmd())
    }};
}
