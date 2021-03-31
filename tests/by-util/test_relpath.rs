use crate::common::util::*;

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

#[test]
fn test_relpath_with_from_no_d() {
    for test in TESTS.iter() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        at.mkdir_all(test.to);
        at.mkdir_all(test.from);

        scene
            .ucmd()
            .arg(test.to)
            .arg(test.from)
            .succeeds()
            .stdout_only(&format!("{}\n", test.expected));
    }
}

#[test]
fn test_relpath_with_from_with_d() {
    for test in TESTS.iter() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let pwd = at.as_string();
        at.mkdir_all(test.to);
        at.mkdir_all(test.from);

        // d is part of subpath
        scene
            .ucmd()
            .arg(test.to)
            .arg(test.from)
            .arg(&format!("-d{}", pwd))
            .succeeds()
            .stdout_only(&format!("{}\n", test.expected));

        // d is not part of subpath
        scene
            .ucmd()
            .arg(test.to)
            .arg(test.from)
            .arg("-d/non_existing")
            .succeeds()
            .stdout_only(&format!("{}/{}\n", pwd, test.to));
    }
}

#[test]
fn test_relpath_no_from_no_d() {
    for test in TESTS.iter() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        at.mkdir_all(test.to);

        scene
            .ucmd()
            .arg(test.to)
            .succeeds()
            .stdout_only(&format!("{}\n", test.to));
    }
}

#[test]
fn test_relpath_no_from_with_d() {
    for test in TESTS.iter() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let pwd = at.as_string();
        at.mkdir_all(test.to);

        // d is part of subpath
        scene
            .ucmd()
            .arg(test.to)
            .arg(&format!("-d{}", pwd))
            .succeeds()
            .stdout_only(&format!("{}\n", test.to));

        // d is not part of subpath
        scene
            .ucmd()
            .arg(test.to)
            .arg("-d/non_existing")
            .succeeds()
            .stdout_only(&format!("{}/{}\n", pwd, test.to));
    }
}
