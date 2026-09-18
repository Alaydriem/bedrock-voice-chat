use common::net::NetTimeouts;

// The whole point of the pair. A client that stops waiting before the server can answer
// replaces a status the player can act on — a refused code, an unadmitted account — with a
// transport error that names no cause.
#[test]
fn the_client_outlasts_the_server_identity_exchange() {
    assert!(
        NetTimeouts::LOGIN > NetTimeouts::IDENTITY,
        "LOGIN ({:?}) must exceed IDENTITY ({:?})",
        NetTimeouts::LOGIN,
        NetTimeouts::IDENTITY
    );
}

// The exchange visits five provider hosts in series. One that hangs must not be able to
// spend the whole budget on its own.
#[test]
fn one_upstream_leg_cannot_consume_the_whole_exchange() {
    assert!(
        NetTimeouts::IDENTITY_UPSTREAM < NetTimeouts::IDENTITY,
        "IDENTITY_UPSTREAM ({:?}) must be shorter than IDENTITY ({:?})",
        NetTimeouts::IDENTITY_UPSTREAM,
        NetTimeouts::IDENTITY
    );
}

// A host that is not there must fail on the connect. The request budgets are sized for
// upstream work, and waiting one out is the "server is down" report that takes half a
// minute to arrive.
#[test]
fn a_connect_gives_up_sooner_than_any_request() {
    assert!(NetTimeouts::CONNECT < NetTimeouts::HTTPS);
    assert!(NetTimeouts::CONNECT < NetTimeouts::LOGIN);
    assert!(NetTimeouts::CONNECT < NetTimeouts::IDENTITY_UPSTREAM);
}

// The screen's reachability verdict must not outlive the request it predicts.
#[test]
fn the_login_budget_covers_the_probe_that_gates_it() {
    assert!(NetTimeouts::LOGIN > NetTimeouts::HTTPS);
}
