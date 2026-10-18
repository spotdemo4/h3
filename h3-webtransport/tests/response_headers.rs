//! Checks the headers a server adds to its response to the CONNECT request.

mod common;

use h3::ext::{Protocol, WebTransportDraft};
use http::{HeaderMap, HeaderValue, StatusCode};

use WebTransportDraft::*;

#[tokio::test]
async fn selects_a_protocol_the_client_offered() {
    let mut request_headers = HeaderMap::new();
    request_headers.insert(
        "wt-available-protocols",
        HeaderValue::from_static(r#""chat-v2", "chat-v1""#),
    );
    let mut response_headers = HeaderMap::new();
    response_headers.insert("wt-protocol", HeaderValue::from_static(r#""chat-v1""#));

    let setup = common::setup_with_headers(
        &WebTransportDraft::ALL,
        &[Draft15],
        Protocol::WEB_TRANSPORT_H3,
        request_headers,
        response_headers,
    )
    .await;

    assert_eq!(
        setup.request.headers()["wt-available-protocols"],
        r#""chat-v2", "chat-v1""#
    );
    assert_eq!(setup.response.status(), StatusCode::OK);
    assert_eq!(setup.response.headers()["wt-protocol"], r#""chat-v1""#);
}

#[tokio::test]
async fn cannot_replace_the_draft_header() {
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "sec-webtransport-http3-draft",
        HeaderValue::from_static("draft07"),
    );

    let setup = common::setup_with_headers(
        &WebTransportDraft::ALL,
        &[Draft02],
        Protocol::WEB_TRANSPORT,
        HeaderMap::new(),
        response_headers,
    )
    .await;

    assert_eq!(setup.session.draft(), Draft02);
    let draft_headers: Vec<_> = setup
        .response
        .headers()
        .get_all("sec-webtransport-http3-draft")
        .iter()
        .collect();
    assert_eq!(draft_headers, ["draft02"]);
}

#[tokio::test]
async fn cannot_add_the_draft_header_to_later_drafts() {
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "sec-webtransport-http3-draft",
        HeaderValue::from_static("draft02"),
    );

    let setup = common::setup_with_headers(
        &WebTransportDraft::ALL,
        &[Draft13],
        Protocol::WEB_TRANSPORT,
        HeaderMap::new(),
        response_headers,
    )
    .await;

    assert_eq!(setup.session.draft(), Draft13);
    assert_eq!(
        setup.response.headers().get("sec-webtransport-http3-draft"),
        None
    );
}
