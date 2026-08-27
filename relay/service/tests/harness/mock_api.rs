use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

// One canned answer, and what it answers to.
pub struct MockRoute {
    method: String,
    path: String,
    query_contains: Option<String>,
    status: StatusCode,
    body: Value,
}

impl MockRoute {
    pub fn new(method: &str, path: &str, body: Value) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            query_contains: None,
            status: StatusCode::OK,
            body,
        }
    }

    // Distinguishes two answers on the same path. The zone walk asks `/zones` twice
    // and the whole point is that the two answers differ.
    pub fn when_query_contains(mut self, fragment: &str) -> Self {
        self.query_contains = Some(fragment.to_string());
        self
    }

    fn matches(&self, method: &Method, uri: &Uri) -> bool {
        if self.method != method.as_str() || self.path != uri.path() {
            return false;
        }

        match &self.query_contains {
            Some(fragment) => uri.query().is_some_and(|q| q.contains(fragment)),
            None => true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub body: String,
}

// A local HTTP server standing in for Cloudflare, Discord, or a DNS-over-HTTPS
// resolver.
//
// The clients under test build their own URLs, choose their own verbs, and read their
// own response shapes. Asserting against recorded requests is the only way to find out
// whether they built the right ones — a hand-rolled fake of the client would assert
// only that the fake matches itself.
pub struct MockApi {
    pub base: String,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

struct MockState {
    routes: Vec<MockRoute>,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl MockApi {
    pub async fn start(routes: Vec<MockRoute>) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = Arc::new(MockState {
            routes,
            requests: Arc::clone(&requests),
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port");
        let addr = listener.local_addr().expect("the bound address");

        let app = axum::Router::new()
            .fallback(Self::respond)
            .with_state(state);
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        Self {
            base: format!("http://{addr}"),
            requests,
        }
    }

    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().expect("the request log").clone()
    }

    async fn respond(
        State(state): State<Arc<MockState>>,
        method: Method,
        uri: Uri,
        body: Bytes,
    ) -> Response {
        state
            .requests
            .lock()
            .expect("the request log")
            .push(RecordedRequest {
                method: method.as_str().to_string(),
                path: uri.path().to_string(),
                body: String::from_utf8_lossy(&body).to_string(),
            });

        match state.routes.iter().find(|r| r.matches(&method, &uri)) {
            Some(route) => (route.status, axum::Json(route.body.clone())).into_response(),
            None => (
                StatusCode::NOT_FOUND,
                axum::Json(json!({ "success": false, "result": [] })),
            )
                .into_response(),
        }
    }
}
