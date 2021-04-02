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

fn convert_path<'a>(path: &'a str) -> Cow<'a, str> {
    #[cfg(windows)]
    return path.replace("/", "\\").into();
    #[cfg(not(windows))]
    return path.into();
}

#[test]
fn test_relpath_with_from_no_d() {
    for test in TESTS.iter() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

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
    for test in TESTS.iter() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let from: &str = &convert_path(test.from);
        let to: &str = &convert_path(test.to);
        let pwd = at.as_string();
        at.mkdir_all(to);
        at.mkdir_all(from);

        // d is part of subpath -> expect relative path
        let mut result = scene
            .ucmd()
            .arg(to)
            .arg(from)
            .arg(&format!("-d{}", pwd))
            .run();
        assert!(result.success);
        // relax rules for windows test environment
        #[cfg(not(windows))]
        assert!(Path::new(&result.stdout).is_relative());

        // d is not part of subpath -> expect absolut path
        result = scene.ucmd().arg(to).arg(from).arg("-dnon_existing").run();
        assert!(result.success);
        assert!(Path::new(&result.stdout).is_absolute());
    }
}

#[test]
fn test_relpath_no_from_no_d() {
    for test in TESTS.iter() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let to: &str = &convert_path(test.to);
        at.mkdir_all(to);

        let result = scene.ucmd().arg(to).run();
        assert!(result.success);
        #[cfg(not(windows))]
        assert_eq!(result.stdout, format!("{}\n", to));
        // relax rules for windows test environment
        #[cfg(windows)]
        assert!(result.stdout.ends_with(&format!("{}\n", to)));
    }
}

#[test]
fn test_relpath_no_from_with_d() {
    for test in TESTS.iter() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let to: &str = &convert_path(test.to);
        let pwd = at.as_string();
        at.mkdir_all(to);

        // d is part of subpath -> expect relative path
        let mut result = scene.ucmd().arg(to).arg(&format!("-d{}", pwd)).run();
        assert!(result.success);
        // relax rules for windows test environment
        #[cfg(not(windows))]
        assert!(Path::new(&result.stdout).is_relative());

        // d is not part of subpath -> expect absolut path
        result = scene.ucmd().arg(to).arg("-dnon_existing").run();
        assert!(result.success);
        assert!(Path::new(&result.stdout).is_absolute());
    }
}
