use ghgrab::github::{GitHubClient, GitHubError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serves a single response and returns its URL. `chunked` omits Content-Length.
async fn serve_once(body: Vec<u8>, chunked: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = socket.read(&mut buf).await;

        if chunked {
            let head = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
            let _ = socket.write_all(head.as_bytes()).await;
            for part in body.chunks(1024) {
                let _ = socket
                    .write_all(format!("{:x}\r\n", part.len()).as_bytes())
                    .await;
                let _ = socket.write_all(part).await;
                if socket.write_all(b"\r\n").await.is_err() {
                    return;
                }
            }
            let _ = socket.write_all(b"0\r\n\r\n").await;
        } else {
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(head.as_bytes()).await;
            let _ = socket.write_all(&body).await;
        }
    });

    format!("http://{}/image.png", addr)
}

fn is_too_large(err: &anyhow::Error) -> bool {
    matches!(err.downcast_ref(), Some(GitHubError::TooLarge(_)))
}

#[tokio::test]
async fn limited_fetch_returns_small_body() {
    let url = serve_once(vec![7u8; 3000], true).await;
    let client = GitHubClient::new(None).unwrap();

    let bytes = client.fetch_bytes_limited(&url, 10_000).await.unwrap();
    assert_eq!(bytes, vec![7u8; 3000]);
}

#[tokio::test]
async fn limited_fetch_allows_body_exactly_at_limit() {
    let url = serve_once(vec![1u8; 4096], true).await;
    let client = GitHubClient::new(None).unwrap();

    let bytes = client.fetch_bytes_limited(&url, 4096).await.unwrap();
    assert_eq!(bytes.len(), 4096);
}

#[tokio::test]
async fn limited_fetch_stops_without_content_length() {
    let url = serve_once(vec![0u8; 64 * 1024], true).await;
    let client = GitHubClient::new(None).unwrap();

    let err = client.fetch_bytes_limited(&url, 10_000).await.unwrap_err();
    assert!(is_too_large(&err), "unexpected error: {:#}", err);
}

#[tokio::test]
async fn limited_fetch_rejects_large_content_length_up_front() {
    let url = serve_once(vec![0u8; 64 * 1024], false).await;
    let client = GitHubClient::new(None).unwrap();

    let err = client.fetch_bytes_limited(&url, 10_000).await.unwrap_err();
    assert!(is_too_large(&err), "unexpected error: {:#}", err);
}
