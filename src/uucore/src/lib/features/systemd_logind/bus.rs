// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Compatibility with logind versions predating the session time/leader APIs.

use async_io::Timer;
use futures_lite::FutureExt;
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use zbus::{Connection, OwnedObjectPath, OwnedVariant};

use crate::utmpx::ReadOptions;

const DESTINATION: &str = "org.freedesktop.login1";

#[derive(Default)]
pub(super) struct SessionDetails {
    pub(super) start_time: Option<u64>,
    pub(super) leader: Option<u32>,
}

pub(super) fn session_details(
    sessions: Vec<(String, ReadOptions)>,
) -> HashMap<String, SessionDetails> {
    session_details_with(sessions, async {
        zbus::connection::Builder::system().build().await
    })
}

pub(super) fn session_details_with(
    sessions: Vec<(String, ReadOptions)>,
    connect: impl Future<Output = zbus::Result<Connection>>,
) -> HashMap<String, SessionDetails> {
    let mut details = HashMap::new();
    if sessions.is_empty() {
        return details;
    }
    zbus::block_on(
        async {
            let Ok(connection) = connect.await else {
                return;
            };
            for (session, options) in sessions {
                if let Some(result) = query(&connection, &session, options).await {
                    details.insert(session, result);
                }
            }
        }
        .or(async {
            // Cancelling the async connection and calls covers authentication
            // as well as every session, without leaving a blocked worker.
            Timer::after(Duration::from_secs(1)).await;
        }),
    );
    details
}

async fn property(connection: &Connection, path: &str, name: &str) -> Option<OwnedVariant> {
    connection
        .call_method(
            Some(DESTINATION),
            path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.freedesktop.login1.Session", name),
        )
        .await
        .ok()?
        .body()
        .deserialize()
        .ok()
}

async fn query(
    connection: &Connection,
    session: &str,
    options: ReadOptions,
) -> Option<SessionDetails> {
    let reply = connection
        .call_method(
            Some(DESTINATION),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "GetSession",
            &(session,),
        )
        .await
        .ok()?;
    let path: OwnedObjectPath = reply.body().deserialize().ok()?;
    let start_time = if options.session_time {
        property(connection, &path, "Timestamp")
            .await
            .and_then(|value| u64::try_from(value).ok())
            .filter(|value| *value > 0)
    } else {
        None
    };
    let leader = if options.session_pid {
        property(connection, &path, "Leader")
            .await
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
    } else {
        None
    };
    Some(SessionDetails { start_time, leader })
}
