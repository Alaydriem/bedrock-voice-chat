use bvc_client_lib::websocket::WebSocketClients;

#[test]
fn lists_a_connection_until_it_is_released() {
    let clients = WebSocketClients::new_shared();
    let id = clients.register("Elgato Stream Deck", "command");
    assert_eq!(clients.snapshot().len(), 1);
    clients.release(id);
    assert!(clients.snapshot().is_empty());
}

// Two Stream Decks are two rows. Keying by name would merge them, and then the first
// disconnect would take both off the table while one was still driving the client.
#[test]
fn keeps_two_connections_with_the_same_name_apart() {
    let clients = WebSocketClients::new_shared();
    let first = clients.register("bvc-cli", "command");
    let second = clients.register("bvc-cli", "command");
    assert_eq!(clients.snapshot().len(), 2);
    clients.release(first);
    assert_eq!(clients.snapshot().len(), 1);
    let _ = second;
}

#[test]
fn counts_commands_against_the_connection_that_sent_them() {
    let clients = WebSocketClients::new_shared();
    let a = clients.register("a", "command");
    let b = clients.register("b", "command");
    clients.count_command(a);
    clients.count_command(a);
    clients.count_command(b);

    let snapshot = clients.snapshot();
    let commands = |name: &str| {
        snapshot
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.commands)
            .unwrap_or(0)
    };
    assert_eq!(commands("a"), 2);
    assert_eq!(commands("b"), 1);
}

// Ids are handed out in sequence, so a count that survived its connection would be
// attributed to whoever got the number next.
#[test]
fn forgets_the_count_when_the_connection_goes() {
    let clients = WebSocketClients::new_shared();
    let id = clients.register("a", "command");
    clients.count_command(id);
    clients.release(id);
    clients.count_command(id);
    assert!(clients.snapshot().is_empty());
}

// A client that sends no User-Agent still has to be findable in the table, and an
// empty cell reads as a rendering bug rather than as a fact about the client.
#[test]
fn names_a_client_that_did_not_introduce_itself() {
    let clients = WebSocketClients::new_shared();
    clients.register("   ", "command");
    assert_eq!(clients.snapshot()[0].name, "Unnamed client");
}
