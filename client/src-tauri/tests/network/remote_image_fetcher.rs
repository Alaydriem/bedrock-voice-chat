use bvc_client_lib::network::RemoteImageFetcher;

use super::local_http_server::LocalHttpServer;

fn image_head(length: Option<usize>) -> String {
    match length {
        Some(length) => format!(
            "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
        ),
        None => "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nConnection: close\r\n\r\n".to_string(),
    }
}

#[tokio::test]
async fn returns_the_body_of_an_image() {
    let port = LocalHttpServer::start(|_| (image_head(Some(4)), b"png!".to_vec()));
    let fetcher = RemoteImageFetcher::new().unwrap();

    let body = fetcher
        .fetch(&format!("http://127.0.0.1:{port}/a.png"))
        .await
        .unwrap();

    assert_eq!(body, b"png!");
}

#[tokio::test]
async fn refuses_a_scheme_that_is_not_http() {
    let fetcher = RemoteImageFetcher::new().unwrap();

    for url in ["httpx://example.com/a.png", "file:///etc/passwd", "ftp://example.com/a.png"] {
        assert!(fetcher.fetch(url).await.is_err(), "{url} must be refused");
    }
}

#[tokio::test]
async fn refuses_a_redirect_to_another_host() {
    let port = LocalHttpServer::start(move |path| {
        if path == "/start" {
            (
                "HTTP/1.1 302 Found\r\nLocation: http://localhost:1/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string(),
                Vec::new(),
            )
        } else {
            (image_head(Some(4)), b"png!".to_vec())
        }
    });
    let fetcher = RemoteImageFetcher::new().unwrap();

    let result = fetcher
        .fetch(&format!("http://127.0.0.1:{port}/start"))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn follows_a_redirect_within_the_same_host() {
    let port = LocalHttpServer::start(|path| {
        if path == "/start" {
            (
                "HTTP/1.1 302 Found\r\nLocation: /final.png\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string(),
                Vec::new(),
            )
        } else {
            (image_head(Some(4)), b"png!".to_vec())
        }
    });
    let fetcher = RemoteImageFetcher::new().unwrap();

    let body = fetcher
        .fetch(&format!("http://127.0.0.1:{port}/start"))
        .await
        .unwrap();

    assert_eq!(body, b"png!");
}

#[tokio::test]
async fn refuses_a_response_that_is_not_an_image() {
    let port = LocalHttpServer::start(|_| {
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 2\r\nConnection: close\r\n\r\n"
                .to_string(),
            b"ok".to_vec(),
        )
    });
    let fetcher = RemoteImageFetcher::new().unwrap();

    let result = fetcher
        .fetch(&format!("http://127.0.0.1:{port}/metadata"))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn refuses_a_declared_length_over_the_cap() {
    let declared = RemoteImageFetcher::MAX_BYTES + 1;
    // The server declares the length and sends one byte: refusing on the header means the
    // fetcher never waits for the rest.
    let port = LocalHttpServer::start(move |_| (image_head(Some(declared)), vec![0u8]));
    let fetcher = RemoteImageFetcher::new().unwrap();

    let result = fetcher
        .fetch(&format!("http://127.0.0.1:{port}/big.png"))
        .await;

    assert!(result.unwrap_err().to_string().contains("larger than"));
}

#[tokio::test]
async fn refuses_an_undeclared_body_that_passes_the_cap_while_reading() {
    let port = LocalHttpServer::start(|_| {
        (
            image_head(None),
            vec![0u8; RemoteImageFetcher::MAX_BYTES + 1024],
        )
    });
    let fetcher = RemoteImageFetcher::new().unwrap();

    let result = fetcher
        .fetch(&format!("http://127.0.0.1:{port}/big.png"))
        .await;

    assert!(result.unwrap_err().to_string().contains("larger than"));
}
