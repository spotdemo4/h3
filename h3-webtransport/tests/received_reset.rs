//! Learning that the peer reset a WebTransport stream without reading from it.

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
async fn bidi_stream_reset_by_client() {
    let setup = setup().await;
    let (mut send, _recv, mut stream) = open_bi(&setup).await;

    assert!(timeout(Duration::from_millis(100), stream.received_reset())
        .await
        .is_err());

    send.reset(VarInt::from_u32(42)).unwrap();
    let reset = timeout(TIMEOUT, stream.received_reset()).await.unwrap();
    assert!(matches!(reset, Ok(Some(42))), "{reset:?}");
}

#[tokio::test]
async fn split_recv_stream_reset_by_client() {
    let setup = setup().await;
    let (mut send, _recv, stream) = open_bi(&setup).await;
    let (_send_half, mut recv_half) = stream.split();

    send.reset(VarInt::from_u32(7)).unwrap();
    let reset = timeout(TIMEOUT, recv_half.received_reset()).await.unwrap();
    assert!(matches!(reset, Ok(Some(7))), "{reset:?}");
}

#[tokio::test]
async fn uni_stream_reset_by_client() {
    let setup = setup().await;
    let mut send = setup.conn.open_uni().await.unwrap();
    let mut header = BytesMut::new();
    write_var(&mut header, 0x54);
    write_var(&mut header, setup.stream.id().into_inner());
    send.write_all(&header).await.unwrap();
    let (_, mut stream) = timeout(TIMEOUT, setup.session.accept_uni())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    send.reset(VarInt::from_u32(3)).unwrap();
    let reset = timeout(TIMEOUT, stream.received_reset()).await.unwrap();
    assert!(matches!(reset, Ok(Some(3))), "{reset:?}");
}

#[tokio::test]
async fn received_reset_fails_when_the_session_closes() {
    let setup = setup().await;
    let (_send, _recv, mut stream) = open_bi(&setup).await;

    let session = setup.session.clone();
    let reset = tokio::spawn(async move { stream.received_reset().await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!reset.is_finished());
    session.close(0, "").await.unwrap();

    let reset = timeout(TIMEOUT, reset).await.unwrap().unwrap();
    assert!(
        matches!(
            reset,
            Err(StreamErrorIncoming::StreamTerminated { error_code }) if error_code == WT_SESSION_GONE
        ),
        "{reset:?}"
    );
}
