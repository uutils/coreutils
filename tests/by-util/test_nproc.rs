use crate::common::util::*;

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
        .ucmd_keepenv()
        .env("OMP_NUM_THREADS", "1")
        .succeeds();

    let nproc_omp: u8 = result.stdout_str().trim().parse().unwrap();
    assert!(nproc - 1 == nproc_omp);

    let result = TestScenario::new(util_name!())
        .ucmd_keepenv()
        .env("OMP_NUM_THREADS", "1") // Has no effect
        .arg("--all")
        .succeeds();
    let nproc_omp: u8 = result.stdout_str().trim().parse().unwrap();
    assert!(nproc == nproc_omp);
}

#[test]
fn test_nproc_ignore() {
    let result = new_ucmd!().succeeds();
    let nproc_total: u8 = result.stdout_str().trim().parse().unwrap();
    if nproc_total > 1 {
        // Ignore all CPU but one
        let result = TestScenario::new(util_name!())
            .ucmd_keepenv()
            .arg("--ignore")
            .arg((nproc_total - 1).to_string())
            .succeeds();
        let nproc: u8 = result.stdout_str().trim().parse().unwrap();
        assert!(nproc == 1);
        // Ignore all CPU but one with a string
        let result = TestScenario::new(util_name!())
            .ucmd_keepenv()
            .arg("--ignore= 1")
            .succeeds();
        let nproc: u8 = result.stdout_str().trim().parse().unwrap();
        assert!(nproc_total - 1 == nproc);
    }
}
