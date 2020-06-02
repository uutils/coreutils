use crate::common::util::*;

#[test]
fn test_nproc() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();
    assert!(result.success);
    let nproc: u8 = result.stdout.trim().parse().unwrap();
    assert!(nproc > 0);
}


#[test]
fn test_nproc_all_omp() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("--all").run();
    assert!(result.success);
    let nproc: u8 = result.stdout.trim().parse().unwrap();
    assert!(nproc > 0);


    let result = TestScenario::new(util_name!())
    .ucmd_keepenv()
    .env("OMP_NUM_THREADS", "1")
    .run();
    assert!(result.success);
    let nproc_omp: u8 = result.stdout.trim().parse().unwrap();
    assert!(nproc-1 == nproc_omp);


    let result = TestScenario::new(util_name!())
    .ucmd_keepenv()
    .env("OMP_NUM_THREADS", "1") // Has no effect
    .arg("--all")
    .run();
    assert!(result.success);
    let nproc_omp: u8 = result.stdout.trim().parse().unwrap();
    assert!(nproc == nproc_omp);
}

#[test]
fn test_nproc_ignore() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();
    assert!(result.success);
    let nproc: u8 = result.stdout.trim().parse().unwrap();
    if nproc > 1 {
        // Ignore all CPU but one
        let result = TestScenario::new(util_name!())
            .ucmd_keepenv()
            .arg("--ignore")
            .arg((nproc - 1).to_string())
            .run();
        assert!(result.success);
        let nproc: u8 = result.stdout.trim().parse().unwrap();
        assert!(nproc == 1);
    }
}
