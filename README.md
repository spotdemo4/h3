# h3 (WebTransport fork)

[![CI](https://github.com/spotdemo4/h3/actions/workflows/CI.yml/badge.svg?branch=master)](https://github.com/spotdemo4/h3/actions/workflows/CI.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[![Chrome](https://img.shields.io/github/check-runs/spotdemo4/h3/master?nameFilter=Chrome&label=Chrome&logo=googlechrome)](https://github.com/spotdemo4/h3/actions/workflows/webtransport-interop.yml)
[![Firefox](https://img.shields.io/github/check-runs/spotdemo4/h3/master?nameFilter=Firefox&label=Firefox&logo=firefoxbrowser)](https://github.com/spotdemo4/h3/actions/workflows/webtransport-interop.yml)
[![Safari](https://img.shields.io/github/check-runs/spotdemo4/h3/master?nameFilter=Safari&label=Safari&logo=safari)](https://github.com/spotdemo4/h3/actions/workflows/webtransport-interop.yml)

[![draft-02](https://img.shields.io/badge/draft--02-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-02)
[![draft-03](https://img.shields.io/badge/draft--03-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-03)
[![draft-04](https://img.shields.io/badge/draft--04-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-04)
[![draft-05](https://img.shields.io/badge/draft--05-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-05)
[![draft-06](https://img.shields.io/badge/draft--06-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-06)
[![draft-07](https://img.shields.io/badge/draft--07-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-07)
[![draft-08](https://img.shields.io/badge/draft--08-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-08)
[![draft-09](https://img.shields.io/badge/draft--09-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-09)
[![draft-10](https://img.shields.io/badge/draft--10-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-10)
[![draft-11](https://img.shields.io/badge/draft--11-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-11)
[![draft-13](https://img.shields.io/badge/draft--13-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-13)
[![draft-14](https://img.shields.io/badge/draft--14-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-14)
[![draft-15](https://img.shields.io/badge/draft--15-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-15)
[![draft-16](https://img.shields.io/badge/draft--16-2ea44f)](https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-http3-16)

This is a fork of [hyperium/h3](https://github.com/hyperium/h3) that makes `h3-webtransport` work with the WebTransport implementations in Chrome, Firefox and Safari, and with clients of newer drafts of [WebTransport over HTTP/3](https://datatracker.ietf.org/doc/draft-ietf-webtrans-http3/). The browser badges show the latest results of interop tests that run against each browser on every push to `master` and every pull request. The draft badges list the drafts a server built on this fork can negotiate.

The changes are meant to go upstream. They address [hyperium/h3#347](https://github.com/hyperium/h3/issues/347) (Safari cannot establish a session) and [hyperium/h3#349](https://github.com/hyperium/h3/issues/349) (no API to close a session).

## What this fork adds

- **Draft negotiation.** A server advertises every supported draft and uses the latest one the client also supports. Choose the drafts with `Builder::webtransport_drafts`; `WebTransportSession::draft()` returns the one in use.
- **Session close and drain.** `WebTransportSession::close(code, reason)`, `drain()`, `closed()` and `draining()` send and receive the `WT_CLOSE_SESSION` and `WT_DRAIN_SESSION` capsules. When a session ends, its streams are reset with `WT_SESSION_GONE`.
- **Fixes:**
  - `WebTransportSession::accept` waits for the client's SETTINGS instead of failing when the CONNECT request arrives first.
  - A session's ID is its CONNECT stream's ID; it used to be wrong on any stream other than 0.
  - A SETTINGS frame can hold every setting a browser sends, and encoding the largest one no longer panics.
- **Browser interop tests** ([`ci/webtransport-interop`](ci/webtransport-interop)) run `examples/webtransport_server.rs` against Chrome and Firefox on Linux and Safari on macOS. They cover streams in both directions, datagrams, drain, and closing from either side.

## WebTransport drafts

The drafts fall into groups that share a SETTINGS codepoint and a wire format:

| Drafts | Negotiated with | `WebTransportDraft` | Clients |
|---|---|---|---|
| 02–06 | `SETTINGS_ENABLE_WEBTRANSPORT` (`0x2b603742`) | `Draft02` | Chrome, Firefox |
| 07–11 | `SETTINGS_WEBTRANSPORT_MAX_SESSIONS` (`0xc671706a`) | `Draft07` | Safari 26 |
| 13–14 | `SETTINGS_WT_MAX_SESSIONS` (`0x14e9cd29`) | `Draft13` | Safari 26 |
| 15–16 | `SETTINGS_WT_ENABLED` (`0x2c7cf000`), upgrade token `webtransport-h3` | `Draft15` | none of these browsers yet |

Not supported:

- Drafts 00 and 01, which no current client speaks.
- Draft 12, which reuses draft 07's codepoint but requires flow control.
- WebTransport flow control in general, so a connection carries one session.
- The `reset_stream_at` QUIC extension, which drafts 10 and later require of the QUIC layer. Quinn doesn't implement it yet ([quinn-rs/quinn#2676](https://github.com/quinn-rs/quinn/issues/2676)).

Safari 26 offers drafts 07 and 13 but only connects if the server advertises draft 07, so keep `Draft07` enabled for Safari.

## Browser notes

- Safari 26's network process crashes when a frame of an unknown type follows `WT_CLOSE_SESSION`, so `h3-webtransport` sends no grease frame on the CONNECT stream (reported to WebKit; the tag [`webkit-wt-close-crash`](https://github.com/spotdemo4/h3/tree/webkit-wt-close-crash) reproduces it).
- Chrome has no `WebTransport.draining`, and Firefox ignores `WT_DRAIN_SESSION`, so only Safari tells the page about a drain.
- Firefox closes the QUIC connection right after a session closes. Await `WebTransportSession::closed()` to learn how the session ended.

## Using the fork

```toml
[dependencies]
h3 = { git = "https://github.com/spotdemo4/h3" }
h3-quinn = { git = "https://github.com/spotdemo4/h3", features = ["datagram"] }
h3-webtransport = { git = "https://github.com/spotdemo4/h3" }
```

See [`examples/webtransport_server.rs`](examples/webtransport_server.rs) for a complete server.

---

The rest of this README is from upstream.

## h3

An async HTTP/3 implementation.

[![CI](https://github.com/hyperium/h3/workflows/CI/badge.svg)](https://github.com/hyperium/h3/actions?query=workflow%3ACI)
[![Discord chat](https://img.shields.io/discord/500028886025895936.svg?logo=discord)](https://discord.gg/q5mVhMD)

This crate provides an [HTTP/3][spec] implementation that is generic over a provided QUIC transport. This allows the project to focus on just HTTP/3, while letting users pick their QUIC implementation based on their specific needs. It includes client and server APIs. Check the original [design][] for more details.

[spec]: https://www.rfc-editor.org/rfc/rfc9114
[design]: docs/PROPOSAL.md

### Status

The `h3` crate is still very experimental. While the client and servers do work, there may still be bugs. And the API could change as we continue to explore. That said, we eagerly welcome contributions, trying it out in test environments, and using at your own risk.

The eventual goal is to use `h3` as an internal dependency of [hyper][].

[hyper]: https://hyper.rs

#### Duvet
This crate uses the [duvet crate][] to check compliance of the [spec][].  
The generated [report][] displays the current status of the requirements of the spec.  

Get more information about this tool in the [contributing][] document.

[duvet crate]: https://crates.io/crates/duvet
[spec]: https://www.rfc-editor.org/rfc/rfc9114
[report]: https://hyper.rs/h3/ci/compliance/report.html#/
[contributing]: CONTRIBUTING.md

### Features

* HTTP/3 client and server implementation
* Async only API
* QUIC transport abstraction via traits in the [`quic`](./h3/src/quic.rs) module
* Runtime independent (h3 does not spawn tasks and works with any runtime)
* Supported QUIC implementations to date are
  [Quinn](https://github.com/quinn-rs/quinn) ([h3-quinn](./h3-quinn/))
  , [s2n-quic](https://github.com/aws/s2n-quic)
  ([s2n-quic-h3](https://github.com/aws/s2n-quic/tree/main/quic/s2n-quic-h3))
  and [MsQuic](https://github.com/microsoft/msquic)
  ([h3-msquic-async](https://github.com/masa-koz/msquic-async-rs/tree/main/h3-msquic-async))

### Overview

* **h3** HTTP/3 implementation
* **h3-quinn** QUIC transport implementation based on [Quinn](https://github.com/quinn-rs/quinn/)

### Getting Started

The [examples](./examples) directory can help get started in two ways:

- There are ready-to-use `client` and `server` binaries to interact with _other_ HTTP/3 peers. Check the README in that directory.
- The source code of those examples can help teach how to use `h3` as either a client or a server.

#### Server

You can find a full server example in [`examples/server.rs`](./examples/server.rs)

#### Client

You can find a full client example in [`examples/client.rs`](./examples/client.rs)

### QUIC Generic

As mentioned, the goal of this library is to be generic over a QUIC implementation. To that effect, integrations with QUIC libraries exist:

- [`h3-quinn`](./h3-quinn/): in this same repository.
- [`s2n-quic-h3`](https://github.com/aws/s2n-quic/tree/main/quic/s2n-quic-h3)
- [`h3-msquic-async`](https://github.com/masa-koz/msquic-async-rs/tree/main/h3-msquic-async)

### Interoperability

This crate as well as the quic implementations are tested ([quinn](https://github.com/quinn-rs/quinn-interop), [s2n-quic](https://github.com/aws/s2n-quic/tree/main/scripts/interop), [MsQuic](https://github.com/microsoft/msquic/tree/main/src/tools/interop)) for interoperability and performance in the [quic-interop-runner](https://github.com/marten-seemann/quic-interop-runner).
You can see the results at (https://interop.seemann.io/).

### License

h3 is provided under the MIT license. See [LICENSE](LICENSE).
