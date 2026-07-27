//! Minimal plain-HTTP stub server for provider tests: records every request
//! (method, path, credential headers, body) and plays back canned JSON keyed
//! by path prefix.

use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Build, Rocket, State};

#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub authorization: Option<String>,
    pub api_user: Option<String>,
    pub api_key: Option<String>,
    pub body: Option<serde_json::Value>,
}

#[derive(Clone, Default)]
pub struct StubState {
    pub requests: Arc<Mutex<Vec<RecordedRequest>>>,
    // path-prefix -> canned JSON body returned with 200; first match wins
    pub responses: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
}

// Credential headers the providers under test are expected to send.
pub struct CapturedHeaders {
    authorization: Option<String>,
    api_user: Option<String>,
    api_key: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CapturedHeaders {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        Outcome::Success(Self {
            authorization: req.headers().get_one("authorization").map(str::to_string),
            api_user: req.headers().get_one("x-api-user").map(str::to_string),
            api_key: req.headers().get_one("x-api-key").map(str::to_string),
        })
    }
}

#[rocket::get("/<path..>")]
async fn stub_get(
    path: std::path::PathBuf,
    headers: CapturedHeaders,
    state: &State<StubState>,
) -> (Status, String) {
    respond("GET", path, headers, None, state)
}

#[rocket::post("/<path..>", data = "<body>")]
async fn stub_post(
    path: std::path::PathBuf,
    headers: CapturedHeaders,
    body: String,
    state: &State<StubState>,
) -> (Status, String) {
    respond("POST", path, headers, Some(body), state)
}

#[rocket::delete("/<path..>")]
async fn stub_delete(
    path: std::path::PathBuf,
    headers: CapturedHeaders,
    state: &State<StubState>,
) -> (Status, String) {
    respond("DELETE", path, headers, None, state)
}

fn respond(
    method: &str,
    path: std::path::PathBuf,
    headers: CapturedHeaders,
    body: Option<String>,
    state: &State<StubState>,
) -> (Status, String) {
    let path = format!("/{}", path.to_string_lossy().replace('\\', "/"));
    state.requests.lock().unwrap().push(RecordedRequest {
        method: method.to_string(),
        path: path.clone(),
        authorization: headers.authorization,
        api_user: headers.api_user,
        api_key: headers.api_key,
        body: body.and_then(|b| serde_json::from_str(&b).ok()),
    });
    let responses = state.responses.lock().unwrap();
    for (prefix, json) in responses.iter() {
        if path.starts_with(prefix.as_str()) {
            return (Status::Ok, json.to_string());
        }
    }
    (Status::Ok, "{}".to_string())
}

pub struct StubServer {
    pub base_url: String,
    pub state: StubState,
    _task: tokio::task::JoinHandle<()>,
}

impl StubServer {
    pub async fn launch() -> Self {
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let state = StubState::default();
        let figment = rocket::Config::figment()
            .merge(("port", port))
            .merge(("address", "127.0.0.1"))
            .merge(("log_level", rocket::config::LogLevel::Off));
        let rocket: Rocket<Build> = rocket::custom(figment)
            .manage(state.clone())
            .mount("/", rocket::routes![stub_get, stub_post, stub_delete]);
        let task = tokio::spawn(async move {
            let _ = rocket.launch().await;
        });
        let base_url = format!("http://127.0.0.1:{}", port);
        let client = reqwest::Client::new();
        for _ in 0..50 {
            if client.get(&base_url).send().await.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        // The readiness probe above is bookkeeping, not test traffic.
        state.requests.lock().unwrap().clear();
        Self {
            base_url,
            state,
            _task: task,
        }
    }
}
