use crate::common::util::*;
use std::borrow::Cow;
use std::path::Path;

struct TestCase<'a> {
    from: &'a str,
    to: &'a str,
    expected: &'a str,
}

const TESTS: [TestCase; 10] = [
    TestCase {
        from: "A/B/C",
        to: "A",
        expected: "../..",
    },
    TestCase {
        from: "A/B/C",
        to: "A/B",
        expected: "..",
    },
    TestCase {
        from: "A/B/C",
        to: "A/B/C",
        expected: "",
    },
    TestCase {
        from: "A/B/C",
        to: "A/B/C/D",
        expected: "D",
    },
    TestCase {
        from: "A/B/C",
        to: "A/B/C/D/E",
        expected: "D/E",
    },
    TestCase {
        from: "A/B/C",
        to: "A/B/D",
        expected: "../D",
    },
    TestCase {
        from: "A/B/C",
        to: "A/B/D/E",
        expected: "../D/E",
    },
    TestCase {
        from: "A/B/C",
        to: "A/D",
        expected: "../../D",
    },
    TestCase {
        from: "A/B/C",
        to: "D/E/F",
        expected: "../../../D/E/F",
    },
    TestCase {
        from: "A/B/C",
        to: "A/D/E",
        expected: "../../D/E",
    },
];

#[allow(clippy::needless_lifetimes)]
fn convert_path<'a>(path: &'a str) -> Cow<'a, str> {
    #[cfg(windows)]
    return path.replace('/', "\\").into();
    #[cfg(not(windows))]
    return path.into();
}

#[test]
fn test_relpath_with_from_no_d() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    for test in &TESTS {
        let from: &str = &convert_path(test.from);
        let to: &str = &convert_path(test.to);
        let expected: &str = &convert_path(test.expected);

        at.mkdir_all(to);
        at.mkdir_all(from);

        scene
            .ucmd()
            .arg(to)
            .arg(from)
            .succeeds()
            .stdout_only(&format!("{}\n", expected));
    }
}

#[test]
fn test_relpath_with_from_with_d() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    for test in &TESTS {
        let from: &str = &convert_path(test.from);
        let to: &str = &convert_path(test.to);
        let pwd = at.as_string();
        at.mkdir_all(to);
        at.mkdir_all(from);

        // d is part of subpath -> expect relative path
        let mut _result_stdout = scene
            .ucmd()
            .arg(to)
            .arg(from)
            .arg(&format!("-d{}", pwd))
            .succeeds()
            .stdout_move_str();
        // relax rules for windows test environment
        #[cfg(not(windows))]
        assert!(Path::new(&_result_stdout).is_relative());

        // d is not part of subpath -> expect absolute path
        _result_stdout = scene
            .ucmd()
            .arg(to)
            .arg(from)
            .arg("-dnon_existing") // spell-checker:disable-line
            .succeeds()
            .stdout_move_str();
        assert!(Path::new(&_result_stdout).is_absolute());
    }
}

#[test]
fn test_relpath_no_from_no_d() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    for test in &TESTS {
        let to: &str = &convert_path(test.to);
        at.mkdir_all(to);

        let _result_stdout = scene.ucmd().arg(to).succeeds().stdout_move_str();
        #[cfg(not(windows))]
        assert_eq!(_result_stdout, format!("{}\n", to));
        // relax rules for windows test environment
        #[cfg(windows)]
        assert!(_result_stdout.ends_with(&format!("{}\n", to)));
    }
}

#[test]
fn test_relpath_no_from_with_d() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    for test in &TESTS {
        let to: &str = &convert_path(test.to);
        let pwd = at.as_string();
        at.mkdir_all(to);

        // d is part of subpath -> expect relative path
        let _result_stdout = scene
            .ucmd()
            .arg(to)
            .arg(&format!("-d{}", pwd))
            .succeeds()
            .stdout_move_str();
        // relax rules for windows test environment
        #[cfg(not(windows))]
        assert!(Path::new(&_result_stdout).is_relative());

        // d is not part of subpath -> expect absolute path
        let result_stdout = scene
            .ucmd()
            .arg(to)
            .arg("-dnon_existing") // spell-checker:disable-line
            .succeeds()
            .stdout_move_str();
        assert!(Path::new(&result_stdout).is_absolute());
    }
}
