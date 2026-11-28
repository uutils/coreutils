// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::*;

#[rstest::rstest]
#[case(0, false)]
#[case(size_of::<utmpx>() - 1, false)]
#[case(size_of::<utmpx>(), true)]
#[case(size_of::<utmpx>() + 1, true)]
#[case(2 * size_of::<utmpx>(), true)]
fn traditional_availability_depends_on_complete_records(
    #[case] length: usize,
    #[case] available: bool,
) {
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(length as u64).unwrap();
    // Empty records still select the traditional reader; their contents do not
    // cause populated utmp files to be replaced by logind sessions.
    assert_eq!(
        has_traditional_records(file.path()),
        available && !cfg!(target_env = "musl")
    );
}

#[test]
fn missing_files_and_directories_have_no_traditional_records() {
    let directory = tempfile::tempdir().unwrap();
    assert!(!has_traditional_records(directory.path()));
    assert!(!has_traditional_records(&directory.path().join("missing")));
}

#[test]
#[cfg(not(target_env = "musl"))]
fn custom_file_selection_persists_and_uses_libc_records() {
    mod support {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/utmpx.rs"
        ));
    }

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("utmp");
    let mut bytes = support::utmpx_record("first");
    bytes.extend(support::utmpx_record("second"));
    // A trailing partial record must not prevent reading complete records.
    bytes.push(0);
    std::fs::write(&path, bytes).unwrap();

    let users = Utmpx::iter_all_records_from(&path)
        .map(|record| record.user())
        .collect::<Vec<_>>();
    assert_eq!(users, ["first", "second"]);
    let users = Utmpx::iter_all_records()
        .map(|record| record.user())
        .collect::<Vec<_>>();
    assert_eq!(users, ["first", "second"]);

    // Explicit empty or missing files must not be replaced by logind records.
    let empty = directory.path().join("empty");
    std::fs::write(&empty, []).unwrap();
    let mut iter = Utmpx::iter_all_records_from(&empty);
    assert!(iter.systemd_iter.is_none());
    assert!(iter.next().is_none());
    drop(iter);
    let mut iter = Utmpx::iter_all_records();
    assert!(iter.systemd_iter.is_none());
    assert!(iter.next().is_none());
    drop(iter);
    let mut iter = Utmpx::iter_all_records_from(directory.path().join("missing"));
    assert!(iter.systemd_iter.is_none());
    assert!(iter.next().is_none());
    drop(iter);
    drop(Utmpx::iter_all_records_from(DEFAULT_FILE));
}
