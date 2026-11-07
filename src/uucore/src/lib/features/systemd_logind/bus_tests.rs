// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::*;
use std::time::{Duration, Instant};

mod bus_fixture {
    include!("../../../../tests/fixtures/logind_bus.rs");
}

fn read(login: &Login, bus: &bus_fixture::Bus, options: ReadOptions) -> Vec<SystemdLoginRecord> {
    read_login_records_with_api(login, options, |sessions| {
        bus::session_details_with(sessions, async {
            zbus::connection::Builder::address(bus.address.as_str())
                .build()
                .await
        })
    })
    .unwrap()
    .into_iter()
    .filter(SystemdLoginRecord::is_user_process)
    .collect()
}

#[rstest::rstest]
#[case("normal", Some(1_700_003_600), Some(4242))]
#[case("bus-unavailable", None, None)]
#[case("bus-method-failure", None, None)]
#[case("bus-path-empty", None, None)]
#[case("bus-property-failure", None, None)]
#[case("bus-variant-failure", None, None)]
#[case("zero-bus-time", None, Some(4242))]
#[case("bus-open-slow", None, None)]
#[case("bus-call-slow", None, None)]
fn unavailable_metadata_preserves_sessions(
    #[case] mode: &str,
    #[case] time: Option<i64>,
    #[case] pid: Option<i32>,
) {
    let directory = tempfile::tempdir().unwrap();
    let bus = bus_fixture::Bus::new(&directory.path().join("bus"), mode);
    let started = Instant::now();
    let records = read(&Login::new(None, None), &bus, ReadOptions::default());
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(records.len(), 1);
    let record = SystemdUtmpxCompat::new(records.into_iter().next().unwrap());
    assert_eq!(record.user(), "user");
    assert_eq!(record.tty_device(), "pts/test");
    assert_eq!(record.host(), "alias.test");
    assert_eq!(
        record
            .try_login_time()
            .map(time::OffsetDateTime::unix_timestamp),
        time
    );
    assert_eq!(record.try_pid(), pid);
}

#[rstest::rstest]
#[case(None, None, false, &["Timestamp"][..])]
#[case(None, None, true, &["Timestamp", "Leader"][..])]
#[case(Some(1_700_000_000_000_000), None, false, &[][..])]
#[case(Some(1_700_000_000_000_000), None, true, &["Leader"][..])]
#[case(None, Some(31337), true, &["Timestamp"][..])]
#[case(Some(0), Some(31337), true, &["Timestamp"][..])]
fn only_missing_required_properties_are_queried(
    #[case] time: Option<u64>,
    #[case] leader: Option<u32>,
    #[case] session_pid: bool,
    #[case] expected: &[&str],
) {
    let directory = tempfile::tempdir().unwrap();
    let bus = bus_fixture::Bus::new(&directory.path().join("bus"), "normal");
    let login = Login::new(time, leader);
    let options = ReadOptions {
        session_time: true,
        session_pid,
    };
    let records = read(&login, &bus, options);
    assert_eq!(records.len(), 1);
    assert_eq!(*login.requested.borrow(), [options]);
    assert_eq!(bus.properties(), expected);
    assert_eq!(bus.connection_count(), usize::from(!expected.is_empty()));
    assert_eq!(
        records[0]
            .login_time
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        if time.is_some_and(|time| time > 0) {
            1_700_000_000
        } else {
            1_700_003_600
        }
    );
    assert_eq!(
        records[0].pid,
        if session_pid {
            leader.unwrap_or(4242)
        } else {
            0
        }
    );
}

#[test]
fn basic_records_do_not_connect_to_dbus() {
    let directory = tempfile::tempdir().unwrap();
    let bus = bus_fixture::Bus::new(&directory.path().join("bus"), "bus-open-slow");
    let login = Login::new(None, None);
    let records = read(&login, &bus, ReadOptions::BASIC);
    assert_eq!(records.len(), 1);
    assert_eq!(*login.requested.borrow(), [ReadOptions::BASIC]);
    assert!(bus.properties().is_empty());
    assert_eq!(bus.connection_count(), 0);
}

#[rstest::rstest]
#[case("normal", 2)]
#[case("bus-open-slow", 0)]
#[case("bus-call-slow", 0)]
#[case("bus-last-slow", 1)]
fn multiple_sessions_survive_shared_timeout(#[case] mode: &str, #[case] known: usize) {
    let directory = tempfile::tempdir().unwrap();
    let bus = bus_fixture::Bus::new(&directory.path().join("bus"), mode);
    let mut login = Login::new(None, None);
    login
        .records
        .push(("second".into(), login.records[0].1.clone()));
    let started = Instant::now();
    let records = read(&login, &bus, ReadOptions::default());
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(records.len(), 2);
    assert_eq!(
        records
            .iter()
            .filter(|r| r.login_time != UNIX_EPOCH)
            .count(),
        known
    );
    assert_eq!(records.iter().filter(|r| r.pid == 4242).count(), known);
}

#[test]
fn retries_recover_in_the_same_process() {
    let directory = tempfile::tempdir().unwrap();
    let bus = bus_fixture::Bus::new(&directory.path().join("bus"), "normal");
    let login = Login::new(None, None);
    for mode in ["bus-open-slow", "bus-call-slow", "bus-unavailable"] {
        bus.set_mode(mode);
        let records = read(&login, &bus, ReadOptions::default());
        assert_eq!(records[0].login_time, UNIX_EPOCH);
        assert_eq!(records[0].pid, 0);
        bus.set_mode("normal");
        let records = read(&login, &bus, ReadOptions::default());
        assert_eq!(
            records[0]
                .login_time
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            1_700_003_600
        );
        assert_eq!(records[0].pid, 4242);
    }
}

#[test]
fn timed_out_authentication_closes_the_socket() {
    use std::io::Read;
    use std::os::unix::net::UnixStream;

    let (client, mut server) = UnixStream::pair().unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let peer = std::thread::spawn(move || {
        // Never answer authentication. The timeout must close the client,
        // allowing this read to reach EOF instead of leaving a connection alive.
        let mut authentication = Vec::new();
        server.read_to_end(&mut authentication).unwrap();
        assert!(!authentication.is_empty());
    });

    let details = bus::session_details_with(
        vec![("legacy".into(), ReadOptions::default())],
        zbus::connection::Builder::unix_stream(client).build(),
    );
    assert!(details.is_empty());
    peer.join().unwrap();
}

#[test]
fn skipped_sessions_do_not_query_dbus() {
    let mut login = Login::new(None, None);
    login.records[0].1.tty.clear();
    let records = read_login_records_with_api(&login, ReadOptions::default(), |requests| {
        assert!(requests.is_empty());
        HashMap::new()
    })
    .unwrap();
    assert!(!records.iter().any(SystemdLoginRecord::is_user_process));
}
