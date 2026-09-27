// zkeys
// Copyright 2025 Julio Merino.
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

//! Mock services for integration tests.

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;
use uuid::Uuid;

/// Query parameters accepted by the mock secret endpoint.
#[derive(Deserialize)]
struct KeyQuery {
    /// Password supplied by the client.
    password: String,
}

/// Request received by the mock service.
#[derive(Debug)]
struct Request {
    /// Key identifier requested by the client.
    key_id: Uuid,

    /// Password supplied by the client.
    password: String,
}

/// State shared by the mock endpoint.
struct MockState {
    /// Status to return to the client.
    status: StatusCode,

    /// Body to return to the client.
    body: String,

    /// Sender used to report the request to the test.
    request_tx: Mutex<Option<oneshot::Sender<Request>>>,
}

/// Handles requests to the mock secret endpoint.
async fn get_secret(
    State(state): State<Arc<MockState>>,
    Path(key_id): Path<Uuid>,
    Query(query): Query<KeyQuery>,
) -> (StatusCode, String) {
    if let Some(request_tx) = state.request_tx.lock().await.take() {
        request_tx.send(Request { key_id, password: query.password }).unwrap();
    }
    (state.status, state.body.clone())
}

/// Mock service running on a loopback TCP port.
pub struct MockService {
    /// URL at which the client can reach the service.
    pub url: String,

    /// Receiver for the request made by the client.
    request_rx: oneshot::Receiver<Request>,

    /// Sender used to stop the mock service.
    shutdown_tx: Option<oneshot::Sender<()>>,

    /// Task running the mock service.
    task: JoinHandle<()>,
}

impl MockService {
    /// Starts a mock service that returns `body` with `status`.
    pub async fn start(status: StatusCode, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (request_tx, request_rx) = oneshot::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let state = Arc::new(MockState {
            status,
            body: body.to_owned(),
            request_tx: Mutex::new(Some(request_tx)),
        });
        let app =
            Router::new().route("/api/v1/keys/{key_id}/secret", get(get_secret)).with_state(state);
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async { shutdown_rx.await.unwrap() })
                .await
                .unwrap();
        });

        Self { url, request_rx, shutdown_tx: Some(shutdown_tx), task }
    }

    /// Verifies the request received from the client.
    pub async fn check_request(&mut self, expected_key_id: Uuid, expected_password: &str) {
        let request = tokio::time::timeout(Duration::from_secs(1), &mut self.request_rx)
            .await
            .expect("Client did not contact mock service")
            .expect("Mock service stopped before receiving a request");
        assert_eq!(expected_key_id, request.key_id);
        assert_eq!(expected_password, request.password);
    }

    /// Stops the mock service and waits for it to exit.
    pub async fn shutdown(mut self) {
        self.shutdown_tx.take().unwrap().send(()).unwrap();
        self.task.await.unwrap();
    }

    /// Returns a loopback URL at which no service is listening.
    pub async fn unreachable_url() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        url
    }
}
