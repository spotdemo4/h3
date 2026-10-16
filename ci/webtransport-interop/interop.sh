#!/usr/bin/env bash
# Runs examples/webtransport_server.rs against a real browser over WebDriver.
#
# Usage: interop.sh <safari|chrome|firefox> [hash|trusted]
#
#   hash     the page passes the certificate's SHA-256 via serverCertificateHashes
#   trusted  the test CA is added to the system trust store (macOS only, needs sudo)
#
# Logs and the JSON result are written to $WT_INTEROP_OUT (default: target/wt-interop).
# $WT_DRAFTS optionally sets the WebTransport drafts the server advertises, e.g. "02,07".
set -euo pipefail

browser=${1:?usage: interop.sh <safari|chrome|firefox> [hash|trusted]}
mode=${2:-hash}
host=${WT_HOST:-localhost}

repo=$(cd "$(dirname "$0")/../.." && pwd)
here="$repo/ci/webtransport-interop"
out=${WT_INTEROP_OUT:-$repo/target/wt-interop}
rm -rf "$out"
mkdir -p "$out"

pids=()
cleanup() {
    for pid in "${pids[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
}
trap cleanup EXIT

"$here/gen-certs.sh" "$out/certs"

if [ "$mode" = trusted ]; then
    case "$(uname)" in
    Darwin)
        sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain "$out/certs/ca.pem"
        ;;
    *)
        echo "trusted mode is only implemented for macOS" >&2
        exit 2
        ;;
    esac
fi

cargo build --example webtransport_server
server=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/debug/examples/webtransport_server

NO_COLOR=1 RUST_LOG=${RUST_LOG:-h3=debug,h3_webtransport=debug,h3_datagram=debug,webtransport_server=debug} \
    "$server" --listen "[::]:4433" --cert "$out/certs/leaf.der" --key "$out/certs/key.der" \
    ${WT_DRAFTS:+--drafts "$WT_DRAFTS"} >"$out/server.log" 2>&1 &
pids+=($!)

python3 -m http.server 8000 --bind 127.0.0.1 --directory "$here" >"$out/http.log" 2>&1 &
pids+=($!)

case "$browser" in
safari)
    safaridriver -p 4444 >"$out/driver.log" 2>&1 &
    ;;
chrome)
    "${CHROMEWEBDRIVER:+$CHROMEWEBDRIVER/}chromedriver" --port=4444 >"$out/driver.log" 2>&1 &
    ;;
firefox)
    "${GECKOWEBDRIVER:+$GECKOWEBDRIVER/}geckodriver" --port 4444 >"$out/driver.log" 2>&1 &
    ;;
*)
    echo "unknown browser $browser" >&2
    exit 2
    ;;
esac
pids+=($!)

for _ in $(seq 1 60); do
    if curl -fsS http://127.0.0.1:4444/status >/dev/null 2>&1 && curl -fsS http://127.0.0.1:8000/index.html >/dev/null 2>&1; then
        break
    fi
    sleep 0.5
done

page="http://127.0.0.1:8000/index.html?url=https://$host:4433/"
if [ "$mode" = hash ]; then
    page="$page&hash=$(cat "$out/certs/leaf.sha256")"
fi

status=0
python3 "$here/run.py" --driver http://127.0.0.1:4444 --browser "$browser" --page "$page" \
    --output "$out/result.json" || status=$?

# Give the server a moment to log the end of the session before stopping it.
sleep 1
cleanup

if [ "$status" = 0 ] && ! grep -q 'Session closed: Ok(CloseInfo { code: 7, reason: "client says bye" })' "$out/server.log"; then
    echo "FAIL client_close: the server did not see the close from the client"
    status=1
fi

echo "::group::SETTINGS and CONNECT seen by the server"
grep -E "Received peer settings|Unsupported setting|Using WebTransport|Established|Session closed|Closing session|ERROR|WARN" "$out/server.log" | head -100 || true
echo "::endgroup::"

exit "$status"
