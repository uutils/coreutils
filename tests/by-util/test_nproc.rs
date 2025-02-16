// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore incorrectnumber
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_nproc() {
    let nproc: u8 = new_ucmd!().succeeds().stdout_str().trim().parse().unwrap();
    assert!(nproc > 0);
}

#[test]
fn test_nproc_all_omp() {
    let result = new_ucmd!().arg("--all").succeeds();

    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert!(nproc > 0);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "60")
        .succeeds();

    let nproc_omp: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc_omp, 60);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "1") // Has no effect
        .arg("--all")
        .succeeds();
    let nproc_omp: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, nproc_omp);

    // If the parsing fails, returns the number of CPU
    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "incorrectnumber") // returns the number CPU
        .succeeds();
    let nproc_omp: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, nproc_omp);
}

#[test]
fn test_nproc_ignore() {
    let result = new_ucmd!().succeeds();
    let nproc_total: u8 = result.stdout_str().trim().parse().unwrap();
    if nproc_total > 1 {
        // Ignore all CPU but one
        let result = TestScenario::new(util_name!())
            .ucmd()
            .arg("--ignore")
            .arg((nproc_total - 1).to_string())
            .succeeds();
        let nproc: u8 = result.stdout_str().trim().parse().unwrap();
        assert_eq!(nproc, 1);
        // Ignore all CPU but one with a string
        let result = TestScenario::new(util_name!())
            .ucmd()
            .arg("--ignore= 1")
            .succeeds();
        let nproc: u8 = result.stdout_str().trim().parse().unwrap();
        assert_eq!(nproc_total - 1, nproc);
    }
}

#[test]
fn test_nproc_ignore_all_omp() {
    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "42")
        .arg("--ignore=40")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 2);
}

#[test]
fn test_nproc_omp_limit() {
    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "42")
        .env("OMP_THREAD_LIMIT", "0")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 42);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "42")
        .env("OMP_THREAD_LIMIT", "2")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 2);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "42")
        .env("OMP_THREAD_LIMIT", "2bad")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 42);

    let result = new_ucmd!().arg("--all").succeeds();
    let nproc_system: u8 = result.stdout_str().trim().parse().unwrap();
    assert!(nproc_system > 0);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_THREAD_LIMIT", "1")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 1);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "0")
        .env("OMP_THREAD_LIMIT", "")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, nproc_system);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "")
        .env("OMP_THREAD_LIMIT", "")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, nproc_system);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "2,2,1")
        .env("OMP_THREAD_LIMIT", "")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(2, nproc);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "2,ignored")
        .env("OMP_THREAD_LIMIT", "")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(2, nproc);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "2,2,1")
        .env("OMP_THREAD_LIMIT", "0")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(2, nproc);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "2,2,1")
        .env("OMP_THREAD_LIMIT", "1bad")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(2, nproc);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "29,2,1")
        .env("OMP_THREAD_LIMIT", "1bad")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(29, nproc);
}
