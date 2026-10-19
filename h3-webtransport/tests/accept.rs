//! `WebTransportSession::accept` hands back requests that do not ask for a session.

mod common;

use std::time::Duration;

use common::{client_h3, endpoints, server_h3};
use h3::ext::{Protocol, WebTransportDraft};
use h3_webtransport::server::{is_webtransport_request, Accepted, WebTransportSession};
use http::{HeaderMap, Method, Request, Response, StatusCode};
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(5);

fn request(method: Method, protocol: Option<Protocol>) -> Request<()> {
    let mut request = Request::builder()
        .method(method)
        .uri("https://localhost/")
        .body(())
        .unwrap();
    if let Some(protocol) = protocol {
        request.extensions_mut().insert(protocol);
    }
    request
}

/// Requests that are not WebTransport CONNECT requests, then one that is
fn requests() -> Vec<(Request<()>, bool)> {
    vec![
        (request(Method::POST, None), false),
        (request(Method::CONNECT, Some(Protocol::WEBSOCKET)), false),
        (
            request(Method::CONNECT, Some(Protocol::WEB_TRANSPORT)),
            true,
        ),
    ]
}

#[tokio::test]
async fn not_webtransport_is_handed_back() {
    let (server, client) = endpoints();
    let addr = server.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let conn = server.accept().await.unwrap().await.unwrap();
        let mut conn = server_h3(conn, WebTransportDraft::ALL.to_vec()).await;
        for (_, is_webtransport) in requests() {
            let (request, stream) = conn
                .accept()
                .await
                .unwrap()
                .unwrap()
                .resolve_request()
                .await
                .unwrap();
            assert_eq!(is_webtransport_request(&request), is_webtransport);

            match WebTransportSession::accept(request, stream, conn, HeaderMap::new())
                .await
                .unwrap()
            {
                Accepted::NotWebTransport {
                    request,
                    mut stream,
                    conn: handed_back,
                } => {
                    assert!(!is_webtransport && !is_webtransport_request(&request));
                    // The caller decides how to respond, and keeps serving the connection.
                    stream
                        .send_response(
                            Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(())
                                .unwrap(),
                        )
                        .await
                        .unwrap();
                    stream.finish().await.unwrap();
                    conn = handed_back;
                }
                Accepted::Session(session) => {
                    assert!(is_webtransport);
                    return session;
                }
            }
        }
        unreachable!("the last request asks for a session");
    });

    let conn = client.connect(addr, "localhost").unwrap().await.unwrap();
    let mut send_request = client_h3(conn, &WebTransportDraft::ALL).await;
    for (request, is_webtransport) in requests() {
        let mut stream = send_request.send_request(request).await.unwrap();
        let response = timeout(TIMEOUT, stream.recv_response())
            .await
            .unwrap()
            .unwrap();
        let expected = if is_webtransport {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        assert_eq!(response.status(), expected);
    }

    let _session = timeout(TIMEOUT, server).await.unwrap().unwrap();
}
