// zkeys
// Copyright 2026 Julio Merino.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
// * Redistributions of source code must retain the above copyright
//   notice, this list of conditions and the following disclaimer.
// * Redistributions in binary form must reproduce the above copyright
//   notice, this list of conditions and the following disclaimer in the
//   documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! `keep-alive` command implementation.

use crate::Config;
use crate::config::Key;
use crate::service::{HttpService, KeepAliveResult, Service};
use log::{error, info};
use std::collections::{HashMap, HashSet};
use std::io;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::Instant;

/// Source of monotonic timestamps.
trait Clock {
    /// Returns the current timestamp.
    fn now(&self) -> Instant;
}

/// Clock backed by the system's monotonic clock.
struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Runtime state for a key being kept alive.
struct ActiveKey {
    /// Key details (identifier, secrets, etc.).
    key: Key,

    /// Time at which to issue the next request.
    next_due: Instant,

    /// Period to retain when a request fails.
    period: Duration,
}

/// Builds the list of keys to refresh.
///
/// If `filter` is specified, this returns information for only the selected key names
/// (after checking that they are defined in the configuration).  Otherwise, this returns
/// all configured keys.
fn select_keys<C: Clock>(
    config: &Config,
    filter: &[String],
    default_period: Duration,
    clock: &C,
) -> io::Result<HashMap<String, ActiveKey>> {
    let mut names = HashSet::new();
    if filter.is_empty() {
        names.extend(config.keys.keys());
    } else {
        names.extend(filter);
    }

    for name in &names {
        if !config.keys.contains_key(*name) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("No key named {name} is configured"),
            ));
        }
    }

    let now = clock.now();
    Ok(names
        .into_iter()
        .map(|name| {
            (
                name.clone(),
                ActiveKey { key: config.keys[name].clone(), next_due: now, period: default_period },
            )
        })
        .collect())
}

/// Processes the result of a keepalive request and updates its key's state.
fn process_keepalive_result<C: Clock>(
    name: &str,
    mut key: ActiveKey,
    result: io::Result<KeepAliveResult>,
    default_period: Duration,
    clock: &C,
) -> Option<ActiveKey> {
    match result {
        Ok(KeepAliveResult::Deleted) => {
            error!("Keepalive for key {name} failed: key was deleted");
            return None;
        }

        Ok(KeepAliveResult::Refreshed(period)) => {
            key.period = period.unwrap_or(default_period);
        }

        Err(error) => {
            error!("Keepalive for key {name} failed: {error}");
        }
    }

    key.next_due = clock.now() + key.period;
    info!("Next keepalive for key {name} is due in {:?}", key.period);
    Some(key)
}

/// Periodically refreshes the requested keys via `service` until all of them are deleted.
async fn keep_alive_internal<S, C>(
    config: Config,
    names: &[String],
    service: io::Result<S>,
    clock: &C,
) -> io::Result<()>
where
    S: Service + Clone + Send + Sync + 'static,
    C: Clock,
{
    let default_period = config.default_key_refresh_period;
    let mut ready_keys: HashMap<_, _> = select_keys(&config, names, default_period, clock)?;
    let mut pending_keys = HashMap::new();
    let service = service?;
    let mut requests = JoinSet::new();

    while !ready_keys.is_empty() || !pending_keys.is_empty() {
        let now = clock.now();
        let due_keys: Vec<_> = ready_keys
            .iter()
            .filter(|(_, key)| key.next_due <= now)
            .map(|(name, _)| name.clone())
            .collect();
        for name in due_keys {
            info!("Sending keepalive for key {name}");
            let key = ready_keys.remove(&name).expect("Due key disappeared");
            let key_id = key.key.id;
            let password = key.key.remote_password.clone();
            pending_keys.insert(name.clone(), key);
            let service = service.clone();
            requests.spawn(async move { (name, service.keep_alive(key_id, &password).await) });
        }

        let next_due = ready_keys.values().map(|key| key.next_due).min();

        let completed = match next_due {
            Some(next_due) if !requests.is_empty() => {
                tokio::select! {
                    completed = requests.join_next() => completed,
                    _ = tokio::time::sleep_until(next_due) => continue,
                }
            }
            Some(next_due) => {
                tokio::time::sleep_until(next_due).await;
                continue;
            }
            None => requests.join_next().await,
        };

        if let Some(completed) = completed {
            let (name, result) = completed.expect("Keepalive request task unexpectedly failed");
            let key = pending_keys.remove(&name).expect("Completed key was not pending");
            if let Some(key) = process_keepalive_result(&name, key, result, default_period, clock) {
                ready_keys.insert(name, key);
            }
        }
    }

    Ok(())
}

/// Periodically refreshes the requested keys until all of them are deleted.
pub async fn keep_alive(config: Config, names: &[String]) -> io::Result<()> {
    let clock = SystemClock;
    let service = HttpService::new(config.service_url.clone());
    keep_alive_internal(config, names, service, &clock).await
}

#[cfg(test)]
mod testutils {
    use super::Clock;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::time::Duration;
    use tokio::time::Instant;

    /// Clock that returns a predefined sequence of timestamps.
    #[derive(Default)]
    pub(super) struct MockClock {
        now: RefCell<VecDeque<Instant>>,
    }

    impl MockClock {
        /// Constructs a mock clock with timestamps at the given offsets from now.
        pub(super) fn new(offsets: &[u64]) -> Self {
            let mut now = VecDeque::with_capacity(offsets.len());
            let base = Instant::now();
            for offset in offsets {
                now.push_back(base + Duration::from_secs(*offset));
            }

            Self { now: RefCell::from(now) }
        }
    }

    impl Drop for MockClock {
        fn drop(&mut self) {
            assert!(self.now.borrow().is_empty(), "Mock timestamps not fully consumed");
        }
    }

    impl Clock for MockClock {
        fn now(&self) -> Instant {
            self.now.borrow_mut().pop_front().expect("No mock timestamps available")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testutils::MockClock;
    use super::*;
    use crate::service::testutils::MockService;
    use std::collections::HashMap;
    use url::Url;
    use uuid::{Uuid, uuid};

    const KEY_ID_1: Uuid = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
    const KEY_ID_2: Uuid = uuid!("0de89eda-f2b5-4cc9-a90c-82dbf96f6983");
    const KEY_ID_3: Uuid = uuid!("c412ad30-5939-4a8d-bad9-9b63a99fda1e");

    /// Returns a configuration with the requested keys and refresh period.
    fn make_config(keys: &[(&str, Uuid, &str)], default_period: Duration) -> Config {
        let keys = keys
            .iter()
            .map(|(name, id, password)| {
                (
                    (*name).to_owned(),
                    Key {
                        id: *id,
                        local_secret: "unused".to_owned(),
                        remote_password: (*password).to_owned(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        Config {
            default_key_refresh_period: default_period,
            keys,
            service_url: Url::parse("http://unused.example/").unwrap(),
            luks: HashMap::default(),
            zfs: HashMap::default(),
        }
    }

    #[tokio::test]
    async fn test_all_keys() {
        let config = make_config(&[("first", KEY_ID_1, "password-1")], Duration::from_secs(30));
        let mut service = MockService::default();
        service.add_keep_alive(KEY_ID_1, "password-1", Ok(KeepAliveResult::Deleted));
        let clock = MockClock::new(&[0, 0]);

        keep_alive_internal(config, &[], Ok(service.clone()), &clock).await.unwrap();
    }

    #[tokio::test]
    async fn test_refresh_periods_and_errors() {
        let config = make_config(
            &[
                ("first", KEY_ID_1, "password-1"),
                ("second", KEY_ID_2, "password-2"),
                ("third", KEY_ID_3, "password-3"),
            ],
            Duration::from_secs(30),
        );
        let mut service = MockService::default();
        service.add_keep_alive(
            KEY_ID_1,
            "password-1",
            Ok(KeepAliveResult::Refreshed(Some(Duration::from_secs(10)))),
        );
        service.add_keep_alive(
            KEY_ID_2,
            "password-2",
            Ok(KeepAliveResult::Refreshed(Some(Duration::from_secs(30)))),
        );
        service.add_keep_alive(
            KEY_ID_3,
            "password-3",
            Ok(KeepAliveResult::Refreshed(Some(Duration::from_secs(20)))),
        );
        service.add_keep_alive(KEY_ID_1, "password-1", Ok(KeepAliveResult::Refreshed(None)));
        service.add_keep_alive(KEY_ID_1, "password-1", Err(io::Error::other("Injected error")));
        service.add_keep_alive(
            KEY_ID_2,
            "password-2",
            Ok(KeepAliveResult::Refreshed(Some(Duration::from_secs(40)))),
        );
        service.add_keep_alive(KEY_ID_3, "password-3", Ok(KeepAliveResult::Refreshed(None)));
        service.add_keep_alive(KEY_ID_3, "password-3", Ok(KeepAliveResult::Deleted));
        service.add_keep_alive(KEY_ID_1, "password-1", Ok(KeepAliveResult::Deleted));
        service.add_keep_alive(KEY_ID_2, "password-2", Ok(KeepAliveResult::Deleted));
        let clock =
            MockClock::new(&[0, 0, 0, 0, 0, 0, 0, 10, 10, 20, 20, 30, 30, 40, 40, 50, 70, 70]);

        keep_alive_internal(config, &[], Ok(service.clone()), &clock).await.unwrap();
    }

    #[tokio::test]
    async fn test_selected_keys() {
        let config = make_config(
            &[("first", KEY_ID_1, "password-1"), ("second", KEY_ID_2, "password-2")],
            Duration::from_secs(30),
        );
        let mut service = MockService::default();
        service.add_keep_alive(KEY_ID_2, "password-2", Ok(KeepAliveResult::Deleted));
        let clock = MockClock::new(&[0, 0]);

        keep_alive_internal(config, &["second".to_owned()], Ok(service.clone()), &clock)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_unknown_name() {
        let config = make_config(&[("first", KEY_ID_1, "password-1")], Duration::from_secs(30));
        let service = MockService::default();
        let clock = MockClock::default();

        let error =
            keep_alive_internal(config, &["unknown".to_owned()], Ok(service.clone()), &clock)
                .await
                .unwrap_err();

        assert_eq!(io::ErrorKind::NotFound, error.kind());
        assert_eq!("No key named unknown is configured", error.to_string());
    }
}
