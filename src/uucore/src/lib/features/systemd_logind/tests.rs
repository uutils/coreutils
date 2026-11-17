// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::*;

#[derive(Default)]
struct Login {
    records: Vec<(String, SessionInfo)>,
    fail: bool,
}

impl Login {
    fn new(time: Option<u64>, leader: Option<u32>) -> Self {
        Self {
            records: vec![(
                "legacy".into(),
                SessionInfo {
                    user: "user".into(),
                    tty: "pts/test".into(),
                    remote_host: "alias.test".into(),
                    start_time: time,
                    leader,
                    ..SessionInfo::default()
                },
            )],
            ..Self::default()
        }
    }
}

impl LoginApi for Login {
    fn sessions(&self) -> UResult<Vec<String>> {
        if self.fail {
            return Err(USimpleError::new(1, "session enumeration failed"));
        }
        Ok(self.records.iter().map(|(id, _)| id.clone()).collect())
    }

    fn session(&self, id: &str) -> Option<SessionInfo> {
        self.records
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, info)| info.clone())
    }
}

#[test]
fn enumeration_errors_are_propagated() {
    let login = Login {
        fail: true,
        ..Login::default()
    };
    assert!(read_login_records_with_api(&login).is_err());
}

#[test]
fn sessions_without_a_terminal_are_skipped() {
    let mut login = Login::new(None, None);
    login.records[0].1.tty.clear();
    let records = read_login_records_with_api(&login).unwrap();
    assert!(!records.iter().any(SystemdLoginRecord::is_user_process));
}

#[test]
fn seat_and_tty_keep_their_metadata() {
    let mut login = Login::new(Some(1_700_000_000_000_000), Some(31337));
    login.records[0].1.seat = "?seat0".into();
    login.records[0].1.tty = "?tty1".into();
    let records = read_login_records_with_api(&login)
        .unwrap()
        .into_iter()
        .filter(SystemdLoginRecord::is_user_process)
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].seat_or_tty, "?seat0");
    assert_eq!(records[1].seat_or_tty, "*tty1");
    assert!(
        records
            .iter()
            .all(|r| r.pid == 31337 && r.host == "alias.test")
    );
}

#[rstest::rstest]
#[case(Some(1_700_000_000_000_000), Some(31337), true, 31337)]
#[case(None, None, false, 0)]
#[case(Some(0), None, false, 0)]
fn session_metadata_is_preserved(
    #[case] time: Option<u64>,
    #[case] leader: Option<u32>,
    #[case] known_time: bool,
    #[case] pid: u32,
) {
    let login = Login::new(time, leader);
    let records = read_login_records_with_api(&login).unwrap();
    let record = records.iter().find(|r| r.is_user_process()).unwrap();
    assert_eq!(record.login_time != UNIX_EPOCH, known_time);
    assert_eq!(record.pid, pid);
    assert_eq!(record.session_leader_pid, pid);
}

#[test]
fn missing_metadata_preserves_legacy_api_types() {
    let record = SystemdLoginRecord {
        session_id: "missing".into(),
        user: "user".into(),
        seat_or_tty: "pts/0".into(),
        raw_device: "pts/0".into(),
        host: String::new(),
        login_time: UNIX_EPOCH,
        pid: 0,
        session_leader_pid: 0,
        record_type: SystemdRecordType::UserProcess,
    };
    let timestamp: SystemTime = record.login_time;
    assert_eq!(timestamp, UNIX_EPOCH);
    let offset: time::OffsetDateTime = record.login_time_offset();
    assert_eq!(offset, time::OffsetDateTime::UNIX_EPOCH);
    assert_eq!(record.try_login_time_offset(), None);
    let compat = SystemdUtmpxCompat::new(record);
    let pid: i32 = compat.pid();
    let timestamp: time::OffsetDateTime = compat.login_time();
    let host: String = compat.canon_host();
    assert_eq!(pid, 0);
    assert_eq!(timestamp, time::OffsetDateTime::UNIX_EPOCH);
    assert!(host.is_empty());
    assert_eq!(compat.try_pid(), None);
    assert_eq!(compat.try_login_time(), None);
}

#[test]
fn test_empty_iterator() {
    let mut iter = SystemdUtmpxIter::empty();

    assert_eq!(iter.len(), 0);
    assert!(iter.is_empty());
    assert!(iter.next().is_none());
    assert!(iter.next_record().is_none());
}

#[test]
fn test_iterator_with_mock_data() {
    // Create iterator with mock records
    let mock_records = vec![
        SystemdLoginRecord {
            session_id: "session1".to_string(),
            user: "user1".to_string(),
            seat_or_tty: "tty1".to_string(),
            raw_device: "tty1".to_string(),
            host: "host1".to_string(),
            login_time: UNIX_EPOCH,
            pid: 1234,
            session_leader_pid: 1234,
            record_type: SystemdRecordType::UserProcess,
        },
        SystemdLoginRecord {
            session_id: "session2".to_string(),
            user: "user2".to_string(),
            seat_or_tty: "pts/0".to_string(),
            raw_device: "pts/0".to_string(),
            host: "host2".to_string(),
            login_time: UNIX_EPOCH,
            pid: 5678,
            session_leader_pid: 5678,
            record_type: SystemdRecordType::UserProcess,
        },
    ];

    let mut iter = SystemdUtmpxIter {
        records: mock_records,
        current_index: 0,
    };

    assert_eq!(iter.len(), 2);
    assert!(!iter.is_empty());

    // Test iterator behavior
    let first = iter.next();
    assert!(first.is_some());

    let second = iter.next();
    assert!(second.is_some());

    let third = iter.next();
    assert!(third.is_none());

    // Iterator should be exhausted
    assert!(iter.next().is_none());
}

#[test]
fn test_get_all_records() {
    let mock_records = vec![SystemdLoginRecord {
        session_id: "session1".to_string(),
        user: "user1".to_string(),
        seat_or_tty: "tty1".to_string(),
        raw_device: "tty1".to_string(),
        host: "host1".to_string(),
        login_time: UNIX_EPOCH,
        pid: 1234,
        session_leader_pid: 1234,
        record_type: SystemdRecordType::UserProcess,
    }];

    let iter = SystemdUtmpxIter {
        records: mock_records,
        current_index: 0,
    };

    let all_records = iter.get_all_records();
    assert_eq!(all_records.len(), 1);
}

#[test]
fn test_systemd_record_conversion() {
    // Test that SystemdLoginRecord converts correctly to SystemdUtmpxCompat
    let record = SystemdLoginRecord {
        session_id: "c1".to_string(),
        user: "testuser".to_string(),
        seat_or_tty: "seat0".to_string(),
        raw_device: "seat0".to_string(),
        host: "localhost".to_string(),
        login_time: UNIX_EPOCH + std::time::Duration::from_secs(1000),
        pid: 9999,
        session_leader_pid: 9999,
        record_type: SystemdRecordType::UserProcess,
    };

    let compat = SystemdUtmpxCompat::new(record);

    // Test the actual conversion logic
    assert_eq!(compat.user(), "testuser");
    assert_eq!(compat.tty_device().as_str(), "seat0");
    assert_eq!(compat.host(), "localhost");
}
