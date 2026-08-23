use bvc_client_lib::websocket::ListenerBinder;
use tokio::net::TcpListener;

const HOST: &str = "127.0.0.1";

// Holds a block of consecutive ports, so a test can describe a range that is genuinely taken
// rather than one port and a hope about its neighbours.
async fn occupy(count: u16) -> (u16, Vec<TcpListener>) {
    for _ in 0..64 {
        let base = TcpListener::bind((HOST, 0)).await.unwrap();
        let first = base.local_addr().unwrap().port();
        let mut held = vec![base];

        for offset in 1..count {
            match TcpListener::bind((HOST, first + offset)).await {
                Ok(listener) => held.push(listener),
                Err(_) => break,
            }
        }

        if held.len() == count as usize {
            return (first, held);
        }
    }

    panic!("no run of {count} consecutive free ports to reserve");
}

#[tokio::test]
async fn a_free_port_is_the_one_it_binds() {
    let (port, held) = occupy(1).await;
    drop(held);

    let listener = ListenerBinder::bind(HOST, port).await.expect("binds");
    assert_eq!(listener.local_addr().unwrap().port(), port);
}

// The reason this exists: a port held by something else used to leave the operator-facing
// listener unbound for the rest of the session.
#[tokio::test]
async fn a_taken_port_moves_to_the_next_free_one() {
    let (port, held) = occupy(1).await;

    let listener = ListenerBinder::bind(HOST, port).await.expect("binds elsewhere");
    let bound = listener.local_addr().unwrap().port();
    assert_ne!(bound, port);
    assert!(bound > port && bound <= port + 16);

    // A port it reports is a port it serves on. Reporting one it cannot accept on would send
    // every plugin to an address that refuses them.
    tokio::net::TcpStream::connect((HOST, bound)).await.expect("the reported port accepts");
    drop(held);
}

#[tokio::test]
async fn the_search_walks_past_a_run_of_taken_ports() {
    let (port, held) = occupy(4).await;

    let listener = ListenerBinder::bind(HOST, port).await.expect("binds past the run");
    assert!(listener.local_addr().unwrap().port() >= port + 4);
    drop(held);
}

// The search is bounded. A machine with no room in the range has to say so, because a listener
// that wandered far from the configured port is not one an operator can find.
#[tokio::test]
async fn an_exhausted_range_is_an_error_rather_than_a_distant_port() {
    let (port, held) = occupy(1).await;

    let outcome = ListenerBinder::bind_within(HOST, port, 0).await;
    assert!(outcome.is_err(), "a span of zero must not look past the preferred port");
    drop(held);
}

// A bind that fails for a reason another port would fail for too must surface as itself.
#[tokio::test]
async fn an_unusable_host_fails_without_searching() {
    let outcome = ListenerBinder::bind("203.0.113.1", 9595).await;
    assert!(outcome.is_err());
}
