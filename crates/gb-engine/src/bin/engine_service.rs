use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::var("GLOWBACK_ENGINE_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8081".to_string());

    let listener = TcpListener::bind(&addr).await?;
    println!("GlowBack engine service listening on {addr}");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            let _ = socket.read(&mut buffer).await;

            let body = r#"{"status":"ok","service":"engine"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });
    }
}
