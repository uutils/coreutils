// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

/// Platform-independent helper for constructing a `PathBuf` from individual elements
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

/// Deduce the name of the test binary from the test filename.
///
/// e.g.: `tests/by-util/test_cat.rs` -> `cat`
#[macro_export]
macro_rules! util_name {
    () => {
        module_path!()
            .split("_")
            .nth(1)
            .and_then(|s| s.split("::").next())
            .expect("no test name")
    };
}

/// Convenience macro for acquiring a [`UCommand`] builder.
///
/// Returns the following:
/// - a [`UCommand`] builder for invoking the binary to be tested
///
/// This macro is intended for quick, single-call tests. For more complex tests
/// that require multiple invocations of the tested binary, see [`TestScenario`]
///
/// [`UCommand`]: crate::tests::common::util::UCommand
/// [`TestScenario]: crate::tests::common::util::TestScenario
#[macro_export]
macro_rules! new_ucmd {
    () => {
        TestScenario::new(util_name!()).ucmd()
    };
}

/// Convenience macro for acquiring a [`UCommand`] builder and a test path.
///
/// Returns a tuple containing the following:
/// - an [`AtPath`] that points to a unique temporary test directory
/// - a [`UCommand`] builder for invoking the binary to be tested
///
/// This macro is intended for quick, single-call tests. For more complex tests
/// that require multiple invocations of the tested binary, see [`TestScenario`]
///
/// [`UCommand`]: crate::tests::common::util::UCommand
/// [`AtPath`]: crate::tests::common::util::AtPath
/// [`TestScenario]: crate::tests::common::util::TestScenario
#[macro_export]
macro_rules! at_and_ucmd {
    () => {{
        let ts = TestScenario::new(util_name!());
        (ts.fixtures.clone(), ts.ucmd())
    }};
}

/// If `common::util::expected_result` returns an error, i.e. the `util` in `$PATH` doesn't
/// include a coreutils version string or the version is too low,
/// this macro can be used to automatically skip the test and print the reason.
#[macro_export]
macro_rules! unwrap_or_return {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(e) => {
                println!("test skipped: {}", e);
                return;
            }
        }
    };
}
