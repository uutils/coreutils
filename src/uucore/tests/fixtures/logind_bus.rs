// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nonblocking

use async_io::Timer;
use futures_lite::{FutureExt, StreamExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use zbus::{ObjectPath, Variant};

// A private D-Bus endpoint speaking the public logind protocol. No system bus,
// daemon executable, or native sd-bus symbols are needed by these tests.
pub struct Bus {
    pub address: String,
    mode: Arc<Mutex<String>>,
    connections: Arc<AtomicUsize>,
    properties: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Bus {
    pub fn new(path: &Path, mode: &str) -> Self {
        let listener = UnixListener::bind(path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let mode = Arc::new(Mutex::new(mode.to_owned()));
        let connections = Arc::new(AtomicUsize::new(0));
        let properties = Arc::new(Mutex::new(Vec::new()));
        let thread = {
            let stop = Arc::clone(&stop);
            let mode = Arc::clone(&mode);
            let connections = Arc::clone(&connections);
            let properties = Arc::clone(&properties);
            std::thread::spawn(move || {
                let mut clients = Vec::new();
                while !stop.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            connections.fetch_add(1, Ordering::Release);
                            let properties = Arc::clone(&properties);
                            let stop = Arc::clone(&stop);
                            let mode = mode.lock().unwrap().clone();
                            clients.push(std::thread::spawn(move || {
                                zbus::block_on(serve(stream, &mode, &properties).or(async {
                                    while !stop.load(Ordering::Acquire) {
                                        Timer::after(Duration::from_millis(5)).await;
                                    }
                                }));
                            }));
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("accept private D-Bus connection: {error}"),
                    }
                }
                for client in clients {
                    client.join().unwrap();
                }
            })
        };
        Self {
            address: format!("unix:path={}", path.display()),
            mode,
            connections,
            properties,
            stop,
            thread: Some(thread),
        }
    }

    pub fn connection_count(&self) -> usize {
        self.connections.load(Ordering::Acquire)
    }

    pub fn properties(&self) -> Vec<String> {
        self.properties.lock().unwrap().clone()
    }

    pub fn set_mode(&self, mode: &str) {
        mode.clone_into(&mut self.mode.lock().unwrap());
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}

async fn serve(stream: UnixStream, mode: &str, properties: &Mutex<Vec<String>>) {
    if mode == "bus-unavailable" {
        return;
    }
    if mode == "bus-open-slow" {
        // Accept the socket but never finish D-Bus authentication.
        std::future::pending::<()>().await;
    }
    let Ok(mut messages) = zbus::connection::Builder::unix_stream(stream)
        .server(zbus::Guid::generate())
        .p2p()
        .unique_name(":1.0")
        .build_message_stream()
        .await
    else {
        return;
    };
    let connection = zbus::Connection::from(&messages);
    async {
        while let Some(Ok(message)) = messages.next().await {
            let header = message.header();
            let member = header.member().map(zbus::names::MemberName::as_str);
            let result = match member {
                Some("Hello") => connection.reply(&header, &":1.1").await,
                Some("GetSession") => {
                    let session: String = message.body().deserialize().unwrap();
                    assert!(session == "legacy" || session == "second");
                    if mode == "bus-call-slow" || (mode == "bus-last-slow" && session == "legacy") {
                        Timer::after(Duration::from_secs(5)).await;
                    }
                    match mode {
                        "bus-method-failure" => {
                            connection
                                .reply_error(
                                    &header,
                                    "org.freedesktop.DBus.Error.Failed",
                                    &"failed",
                                )
                                .await
                        }
                        "bus-path-empty" => connection.reply(&header, &"").await,
                        _ => {
                            connection
                                .reply(&header, &ObjectPath::try_from("/session/legacy").unwrap())
                                .await
                        }
                    }
                }
                Some("Get") => {
                    let (interface, property): (String, String) =
                        message.body().deserialize().unwrap();
                    assert_eq!(interface, "org.freedesktop.login1.Session");
                    properties.lock().unwrap().push(property.clone());
                    if mode == "bus-property-failure" {
                        connection
                            .reply_error(&header, "org.freedesktop.DBus.Error.Failed", &"failed")
                            .await
                    } else {
                        let value = match (mode, property.as_str()) {
                            ("bus-variant-failure", _) => Variant::from("wrong type"),
                            ("zero-bus-time", "Timestamp") => Variant::from(0_u64),
                            (_, "Timestamp") => Variant::from(1_700_003_600_000_000_u64),
                            (_, "Leader") => Variant::from(4242_u32),
                            _ => panic!("unexpected property {property}"),
                        };
                        connection.reply(&header, &value).await
                    }
                }
                _ => panic!("unexpected D-Bus member {member:?}"),
            };
            if result.is_err() {
                break;
            }
        }
    }
    .await;
}
