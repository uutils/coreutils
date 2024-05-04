use uucore::get_uu_log_enabled;

#[test]
fn test_uu_log_disabled() {
    assert_eq!(get_uu_log_enabled(), false);
}
