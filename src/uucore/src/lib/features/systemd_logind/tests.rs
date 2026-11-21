// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::*;
use std::cell::RefCell;

#[derive(Default)]
struct Login {
    records: Vec<(String, SessionInfo)>,
    requested: RefCell<Vec<ReadOptions>>,
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

    fn session(&self, id: &str, options: ReadOptions) -> Option<SessionInfo> {
        self.requested.borrow_mut().push(options);
        let mut info = self.records.iter().find(|(name, _)| name == id)?.1.clone();
        if !options.session_time {
            info.start_time = None;
        }
        if !options.session_pid {
            info.leader = None;
        }
        Some(info)
    }
}

#[test]
fn enumeration_errors_are_propagated() {
    let login = Login {
        fail: true,
        ..Login::default()
    };
    assert!(read_login_records_with_api(&login, ReadOptions::default()).is_err());
}

#[test]
fn sessions_without_a_terminal_are_skipped() {
    let mut login = Login::new(None, None);
    login.records[0].1.tty.clear();
    let records = read_login_records_with_api(&login, ReadOptions::default()).unwrap();
    assert!(!records.iter().any(SystemdLoginRecord::is_user_process));
}

#[test]
fn seat_and_tty_keep_their_metadata() {
    let mut login = Login::new(Some(1_700_000_000_000_000), Some(31337));
    login.records[0].1.seat = "?seat0".into();
    login.records[0].1.tty = "?tty1".into();
    let records = read_login_records_with_api(&login, ReadOptions::default())
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
#[case(
    Some(1_700_000_000_000_000),
    Some(31337),
    ReadOptions::default(),
    true,
    31337
)]
#[case(None, None, ReadOptions::default(), false, 0)]
#[case(Some(0), None, ReadOptions::default(), false, 0)]
#[case(Some(1_700_000_000_000_000), Some(31337), ReadOptions::BASIC, false, 0)]
#[case(Some(1_700_000_000_000_000), Some(31337), ReadOptions { session_time: true, session_pid: false }, true, 0)]
fn requested_session_metadata_is_preserved(
    #[case] time: Option<u64>,
    #[case] leader: Option<u32>,
    #[case] options: ReadOptions,
    #[case] known_time: bool,
    #[case] pid: u32,
) {
    let login = Login::new(time, leader);
    let records = read_login_records_with_api(&login, options).unwrap();
    let record = records.iter().find(|r| r.is_user_process()).unwrap();
    assert_eq!(*login.requested.borrow(), [options]);
    assert_eq!(record.login_time != UNIX_EPOCH, known_time);
    assert_eq!(record.pid, pid);
    assert_eq!(record.session_leader_pid, pid);
}
