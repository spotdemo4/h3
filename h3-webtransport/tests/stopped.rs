//! Learning that the peer stopped a WebTransport stream without writing to it.

mod common;

use std::time::Duration;

use bytes::BytesMut;
use common::{setup, write_var, Setup};
use h3::quic::{BidiStream as _, StreamErrorIncoming};
use h3_webtransport::{server::AcceptedBi, stream::BidiStream, WT_SESSION_GONE};
use quinn::VarInt;
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(5);

type ServerBidi = BidiStream<h3_quinn::BidiStream<bytes::Bytes>, bytes::Bytes>;

/// Opens a WebTransport bidirectional stream from the client
async fn open_bi(setup: &Setup) -> (quinn::SendStream, quinn::RecvStream, ServerBidi) {
    let (mut send, recv) = setup.conn.open_bi().await.unwrap();
    let mut header = BytesMut::new();
    write_var(&mut header, 0x41);
    write_var(&mut header, setup.stream.id().into_inner());
    send.write_all(&header).await.unwrap();

    let resolver = setup.session.accept_bi().await.unwrap().unwrap();
    let AcceptedBi::BidiStream(_, stream) = resolver.resolve().await.unwrap() else {
        panic!("expected a WebTransport stream");
    };
    (send, recv, stream)
}

#[tokio::test]
async fn bidi_stream_stopped_by_client() {
    let setup = setup().await;
    let (_send, mut recv, mut stream) = open_bi(&setup).await;

    assert!(timeout(Duration::from_millis(100), stream.stopped())
        .await
        .is_err());

    recv.stop(VarInt::from_u32(42)).unwrap();
    let stopped = timeout(TIMEOUT, stream.stopped()).await.unwrap();
    assert!(matches!(stopped, Ok(Some(42))), "{stopped:?}");
}

#[tokio::test]
async fn split_send_stream_stopped_by_client() {
    let setup = setup().await;
    let (_send, mut recv, stream) = open_bi(&setup).await;
    let (mut send_half, _recv_half) = stream.split();

    recv.stop(VarInt::from_u32(7)).unwrap();
    let stopped = timeout(TIMEOUT, send_half.stopped()).await.unwrap();
    assert!(matches!(stopped, Ok(Some(7))), "{stopped:?}");
}

#[tokio::test]
async fn stopped_fails_when_the_session_closes() {
    let setup = setup().await;
    let (_send, _recv, mut stream) = open_bi(&setup).await;

    let session = setup.session.clone();
    let stopped = tokio::spawn(async move { stream.stopped().await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!stopped.is_finished());
    session.close(0, "").await.unwrap();

    let stopped = timeout(TIMEOUT, stopped).await.unwrap().unwrap();
    assert!(
        matches!(
            stopped,
            Err(StreamErrorIncoming::StreamTerminated { error_code }) if error_code == WT_SESSION_GONE
        ),
        "{stopped:?}"
    );
}
