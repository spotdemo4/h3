# H3 Datagram

This is `h3-datagram` from the [spotdemo4/h3](https://github.com/spotdemo4/h3) fork, published as `trev-h3-datagram` for use with [`trev-h3`](https://crates.io/crates/trev-h3). The library is still named `h3_datagram`.

this crate provides an implementation of the [h3-datagram](https://datatracker.ietf.org/doc/html/rfc9297) spec that works with the h3 crate.

# Status
This crate is still in experimental. The API is subject to change. It may contain bugs and is not yet complete. Use with caution.

## Usage
As stated in the [rfc](https://datatracker.ietf.org/doc/html/rfc9297#abstract) this is intended to be used for protocol extensions like [Web-Transport](https://datatracker.ietf.org/doc/draft-ietf-webtrans-http3/) and not directly by applications.

> HTTP Datagrams and the Capsule Protocol are intended for use by HTTP extensions, not applications.