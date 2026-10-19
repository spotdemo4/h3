//! Errors reported through `AsyncRead` and `AsyncWrite` keep their meaning.

mod common;

use std::{io, time::Duration};

use bytes::BytesMut;
use common::{setup, write_var, Setup};
use h3::quic::StreamErrorIncoming;
use h3_webtransport::{server::AcceptedBi, stream::BidiStream, WT_SESSION_GONE};
use quinn::VarInt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

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

/// Returns the `StreamErrorIncoming` kept by an I/O error
fn inner(error: &io::Error) -> &StreamErrorIncoming {
    error
        .get_ref()
        .and_then(|e| e.downcast_ref::<StreamErrorIncoming>())
        .unwrap_or_else(|| panic!("no StreamErrorIncoming in {error:?}"))
}

#[tokio::test]
async fn write_after_stop_sending() {
    let setup = setup().await;
    let (_send, mut recv, mut stream) = open_bi(&setup).await;

    recv.stop(VarInt::from_u32(42)).unwrap();
    timeout(TIMEOUT, stream.stopped()).await.unwrap().unwrap();

    let error = stream.write_all(b"response").await.unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::ConnectionReset);
    assert!(matches!(
        inner(&error),
        StreamErrorIncoming::StreamTerminated { error_code: 42 }
    ));
}

#[tokio::test]
async fn read_after_reset() {
    let setup = setup().await;
    let (mut send, _recv, mut stream) = open_bi(&setup).await;

    send.reset(VarInt::from_u32(7)).unwrap();
    let error = timeout(TIMEOUT, stream.read_to_end(&mut Vec::new()))
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::ConnectionReset);
    assert!(matches!(
        inner(&error),
        StreamErrorIncoming::StreamTerminated { error_code: 7 }
    ));
}

#[tokio::test]
async fn write_after_session_close() {
    let setup = setup().await;
    let (_send, _recv, mut stream) = open_bi(&setup).await;

    setup.session.close(0, "").await.unwrap();
    let error = stream.write_all(b"response").await.unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::ConnectionReset);
    assert!(matches!(
        inner(&error),
        StreamErrorIncoming::StreamTerminated { error_code } if *error_code == WT_SESSION_GONE
    ));
}

#[tokio::test]
async fn write_after_connection_close() {
    let setup = setup().await;
    let (_send, _recv, mut stream) = open_bi(&setup).await;

    setup.conn.close(VarInt::from_u32(0x100), b"");
    let error = timeout(TIMEOUT, async {
        loop {
            if let Err(error) = stream.write_all(b"response").await {
                break error;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::ConnectionAborted);
    assert!(matches!(
        inner(&error),
        StreamErrorIncoming::ConnectionErrorIncoming { .. }
    ));
}
