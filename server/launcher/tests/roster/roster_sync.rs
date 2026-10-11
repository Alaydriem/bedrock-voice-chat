use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use bvc_launcher::roster::{RosterSync, RosterTick};
use serde_json::Value;

use crate::support::{HttpFixture, HttpReply, RecordedRequest};

/// A BDS directory with `variables.json` pointing at a local HTTP fixture standing in for BVC.
struct Roster {
    dir: tempfile::TempDir,
    http: HttpFixture,
}

impl Roster {
    fn new(names: &[&str]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let http = HttpFixture::start();
        let base = http.url("");
        fs::write(
            dir.path().join("variables.json"),
            format!(r#"{{"bvc_server":"{base}","bvc_access_token":"bvc_test_secret"}}"#),
        )
        .unwrap();
        let roster = Self { dir, http };
        roster.allow(names);
        roster
    }

    fn allow(&self, names: &[&str]) {
        let entries: Vec<String> = names
            .iter()
            .map(|n| format!(r#"{{"ignoresPlayerLimit":false,"name":"{n}"}}"#))
            .collect();
        fs::write(self.path("allowlist.json"), format!("[{}]", entries.join(","))).unwrap();
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn sync(&self) -> RosterSync {
        RosterSync::new(
            self.path("variables.json"),
            self.path("allowlist.json"),
            Duration::from_secs(60),
        )
    }

    fn ready(&self) {
        self.http
            .route("/health/readiness", HttpReply::Body(b"{}".to_vec()));
        self.http.route("/api/position", HttpReply::Body(Vec::new()));
    }

    fn posts(&self) -> Vec<RecordedRequest> {
        self.http
            .requests()
            .into_iter()
            .filter(|r| r.method == "POST" && r.path == "/api/position")
            .collect()
    }

    fn names(request: &RecordedRequest) -> Vec<String> {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        body["players"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap().to_string())
            .collect()
    }
}

#[test]
fn nothing_is_posted_until_bvc_is_ready() {
    let roster = Roster::new(&["Alaydriem"]);
    roster
        .http
        .route("/health/readiness", HttpReply::Status(503));

    assert_eq!(
        roster.sync().tick(),
        RosterTick::NotReady("BVC answered 503 Service Unavailable".to_string())
    );
    assert!(roster.posts().is_empty());
}

// A host that cannot reach BVC at all must say why, or it retries silently for ever.
#[test]
fn an_unreachable_bvc_reports_the_connection_error() {
    let roster = Roster::new(&["Alaydriem"]);
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    fs::write(
        roster.path("variables.json"),
        format!(r#"{{"bvc_server":"http://127.0.0.1:{port}","bvc_access_token":"t"}}"#),
    )
    .unwrap();

    let RosterTick::NotReady(reason) = roster.sync().tick() else {
        panic!("an unreachable BVC must not be ready");
    };
    assert!(!reason.is_empty());
}

// The server answers 200 even when it could not create the player, so a name is sent again
// once an hour rather than trusted for the life of the launcher.
#[test]
fn every_name_is_sent_again_once_an_hour() {
    let roster = Roster::new(&["Alaydriem"]);
    roster.ready();
    let mut sync = RosterSync::new(
        roster.path("variables.json"),
        roster.path("allowlist.json"),
        Duration::from_secs(30 * 60),
    );

    assert_eq!(sync.tick(), RosterTick::Registered(1));
    assert_eq!(sync.tick(), RosterTick::NothingPending);
    assert_eq!(sync.tick(), RosterTick::Registered(1));
}

#[test]
fn the_first_ready_tick_posts_every_name_and_the_next_posts_nothing() {
    let roster = Roster::new(&["Alaydriem", "AlayCamera"]);
    roster.ready();
    let mut sync = roster.sync();

    assert_eq!(sync.tick(), RosterTick::Registered(2));
    assert_eq!(sync.tick(), RosterTick::NothingPending);

    let posts = roster.posts();
    assert_eq!(posts.len(), 1);
    assert_eq!(Roster::names(&posts[0]), ["Alaydriem", "AlayCamera"]);
}

#[test]
fn an_added_name_is_posted_alone_and_a_removed_one_is_ignored() {
    let roster = Roster::new(&["Alaydriem", "AlayCamera"]);
    roster.ready();
    let mut sync = roster.sync();
    sync.tick();

    roster.allow(&["Alaydriem", "Newcomer"]);

    assert_eq!(sync.tick(), RosterTick::Registered(1));
    assert_eq!(Roster::names(&roster.posts()[1]), ["Newcomer"]);
}

#[test]
fn a_rejected_post_is_retried_on_the_next_tick() {
    let roster = Roster::new(&["Alaydriem"]);
    roster
        .http
        .route("/health/readiness", HttpReply::Body(b"{}".to_vec()));
    roster.http.route("/api/position", HttpReply::Status(401));
    let mut sync = roster.sync();

    assert!(matches!(sync.tick(), RosterTick::Rejected(_)));

    roster.http.route("/api/position", HttpReply::Body(Vec::new()));
    assert_eq!(sync.tick(), RosterTick::Registered(1));
}

#[test]
fn players_are_sent_as_death_dimension_spectators_with_the_token() {
    let roster = Roster::new(&["Alaydriem"]);
    roster.ready();

    roster.sync().tick();

    let post = &roster.posts()[0];
    assert_eq!(post.header("authorization"), Some("Bearer bvc_test_secret"));
    let body: Value = serde_json::from_slice(&post.body).unwrap();
    assert_eq!(body["game"], "minecraft");
    let player = &body["players"][0];
    assert_eq!(player["dimension"], "death");
    assert_eq!(player["spectator"], true);
    assert_eq!(player["deafen"], false);
    assert!(player.get("world_uuid").is_none());
}

#[test]
fn a_missing_allowlist_is_reported_and_sends_nothing() {
    let roster = Roster::new(&[]);
    fs::remove_file(roster.path("allowlist.json")).unwrap();
    roster.ready();

    assert_eq!(roster.sync().tick(), RosterTick::NoAllowlist);
    assert!(roster.posts().is_empty());
}

#[test]
fn missing_variables_send_nothing() {
    let roster = Roster::new(&["Alaydriem"]);
    fs::write(roster.path("variables.json"), "{}").unwrap();
    roster.ready();

    assert_eq!(roster.sync().tick(), RosterTick::NoEndpoint);
    assert!(roster.http.requests().is_empty());
}
