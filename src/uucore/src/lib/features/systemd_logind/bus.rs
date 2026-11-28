// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! The static musl backend queries logind without loading a shared library.

use std::collections::HashMap;
use std::time::Duration;

use zbus::connection::Builder;
use zbus::{OwnedObjectPath, OwnedVariant};
use zruntime::SharedRuntime;

use super::{LoginApi, SessionInfo};
use crate::error::UResult;

const DESTINATION: &str = "org.freedesktop.login1";
const SESSION_INTERFACE: &str = "org.freedesktop.login1.Session";
const READ_TIMEOUT: Duration = Duration::from_secs(1);

type ListedSession = (String, u32, String, String, OwnedObjectPath);

pub(super) struct BusLogin(HashMap<String, SessionInfo>);

impl BusLogin {
    pub(super) fn connect(builder: Builder<'_>) -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = SharedRuntime::current()?;
        // Bound connection setup and all session requests with one deadline.
        Ok(runtime.block_on(runtime.timeout(READ_TIMEOUT, Self::read_sessions(builder)))??)
    }

    async fn read_sessions(builder: Builder<'_>) -> zbus::Result<Self> {
        let connection = builder.build().await?;
        let reply = connection
            .call_method(
                Some(DESTINATION),
                "/org/freedesktop/login1",
                Some("org.freedesktop.login1.Manager"),
                "ListSessions",
                &(),
            )
            .await?;
        let sessions: Vec<ListedSession> = reply.body().deserialize()?;
        let mut records = HashMap::with_capacity(sessions.len());
        for (id, uid, user, seat, path) in sessions {
            let reply = match connection
                .call_method(
                    Some(DESTINATION),
                    path.as_str(),
                    Some("org.freedesktop.DBus.Properties"),
                    "GetAll",
                    &SESSION_INTERFACE,
                )
                .await
            {
                Ok(reply) => reply,
                // A session can end while we are reading its properties.
                Err(zbus::Error::FDO(error))
                    if matches!(error.as_ref(), zbus::fdo::Error::UnknownObject(_)) =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            let mut properties: HashMap<String, OwnedVariant> = reply.body().deserialize()?;
            let mut string = |name: &str| {
                properties
                    .remove(name)
                    .and_then(|value| String::try_from(value).ok())
                    .unwrap_or_default()
            };
            records.insert(
                id,
                SessionInfo {
                    user: if user.is_empty() {
                        uid.to_string()
                    } else {
                        user
                    },
                    tty: string("TTY"),
                    seat,
                    remote_host: string("RemoteHost"),
                    display: string("Display"),
                    start_time: properties
                        .remove("Timestamp")
                        .and_then(|value| u64::try_from(value).ok()),
                    leader: properties
                        .remove("Leader")
                        .and_then(|value| u32::try_from(value).ok())
                        .filter(|&pid| pid > 0),
                },
            );
        }
        Ok(Self(records))
    }
}

impl LoginApi for BusLogin {
    fn sessions(&self) -> UResult<Vec<String>> {
        Ok(self.0.keys().cloned().collect())
    }

    fn session(&self, id: &str) -> Option<SessionInfo> {
        self.0.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn an_unresponsive_bus_times_out_during_authentication() {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("bus");
        // Accept no traffic, so the client will connect but receive no response.
        let _listener = UnixListener::bind(&socket).unwrap();
        let address = format!("unix:path={}", socket.display());
        let builder = Builder::address(address.as_str());
        let error = BusLogin::connect(builder).err().unwrap();
        assert!(error.is::<zruntime::TimedOut>(), "{error:?}");
    }
}
