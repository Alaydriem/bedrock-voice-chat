use bvc_client_lib::ApiClient;

use super::client_material::ClientMaterial;
use super::local_tls_server::LocalTlsServer;
use super::play_chain::PlayChain;

const PLAY_NAME: &str = "a-b-c.play.bedrockvc.stream";

#[tokio::test]
async fn the_api_client_reaches_a_play_named_server_with_a_play_root_and_strict_mode() {
    let chain = PlayChain::new("Test Play Root", PLAY_NAME);
    let root = chain.root.clone();
    let port = LocalTlsServer::start(chain);
    let material = ClientMaterial::new();
    let client = ApiClient::new_strict_with_play_roots(
        material.ca_pem,
        material.identity_pem,
        vec![root],
    );

    assert_eq!(
        LocalTlsServer::get(&client, PLAY_NAME, port).await.unwrap(),
        200
    );
}

// The control for the test above: the same server, with a client whose play roots do not
// include the one that signed it, must be refused. Without this the first test would pass
// against a client that accepts anything.
#[tokio::test]
async fn the_api_client_refuses_a_play_named_server_signed_by_another_root() {
    let served = PlayChain::new("Test Play Root", PLAY_NAME);
    let other = PlayChain::new("Other Play Root", PLAY_NAME);
    let port = LocalTlsServer::start(served);
    let material = ClientMaterial::new();
    let client = ApiClient::new_strict_with_play_roots(
        material.ca_pem,
        material.identity_pem,
        vec![other.root],
    );

    assert!(LocalTlsServer::get(&client, PLAY_NAME, port).await.is_err());
}
