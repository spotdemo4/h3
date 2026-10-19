//! Accepting WebTransport unidirectional streams opened by the client.

mod common;

use std::time::Duration;

use bytes::BytesMut;
use common::{setup, write_var, Setup};
use tokio::{io::AsyncReadExt, time::timeout};

const TIMEOUT: Duration = Duration::from_secs(5);

/// Opens a WebTransport unidirectional stream from the client and sends its header
async fn open_uni(setup: &Setup) -> quinn::SendStream {
    let mut send = setup.conn.open_uni().await.unwrap();
    let mut header = BytesMut::new();
    write_var(&mut header, 0x54);
    write_var(&mut header, setup.stream.id().into_inner());
    send.write_all(&header).await.unwrap();
    send
}

#[tokio::test]
async fn accept_before_payload() {
    let setup = setup().await;
    let mut send = open_uni(&setup).await;

    // The client sends nothing after the header until the stream is accepted.
    let (id, mut stream) = timeout(TIMEOUT, setup.session.accept_uni())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(id, setup.session.session_id());

    send.write_all(b"hello").await.unwrap();
    send.finish().unwrap();
    let mut payload = Vec::new();
    timeout(TIMEOUT, stream.read_to_end(&mut payload))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(payload, b"hello");
}

#[tokio::test]
async fn accept_empty() {
    let setup = setup().await;
    let mut send = open_uni(&setup).await;
    send.finish().unwrap();

    let (_, mut stream) = timeout(TIMEOUT, setup.session.accept_uni())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let mut payload = Vec::new();
    timeout(TIMEOUT, stream.read_to_end(&mut payload))
        .await
        .unwrap()
        .unwrap();
    assert!(payload.is_empty());
}
