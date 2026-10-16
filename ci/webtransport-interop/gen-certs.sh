#!/usr/bin/env bash
# Generates a CA and an ECDSA P-256 leaf certificate for localhost in the given directory.
#
# The leaf is valid for 10 days so it can also be used with serverCertificateHashes, which
# requires a validity period of at most 14 days. Outputs:
#   ca.pem             CA certificate, for adding to a trust store
#   leaf.der, key.der  DER certificate and PKCS#8 key for examples/webtransport_server.rs
#   leaf.sha256        hex SHA-256 of leaf.der, for serverCertificateHashes
set -euo pipefail

out=${1:?usage: gen-certs.sh <dir>}
mkdir -p "$out"
cd "$out"

cat >ca.cnf <<'EOF'
[req]
distinguished_name = dn
prompt = no
[dn]
CN = h3 interop test CA
[ext]
basicConstraints = critical,CA:TRUE
keyUsage = critical,keyCertSign,cRLSign
subjectKeyIdentifier = hash
EOF

cat >leaf.cnf <<'EOF'
[req]
distinguished_name = dn
prompt = no
[dn]
CN = localhost
[ext]
basicConstraints = critical,CA:FALSE
keyUsage = critical,digitalSignature
extendedKeyUsage = serverAuth
subjectAltName = DNS:localhost,IP:127.0.0.1,IP:::1
authorityKeyIdentifier = keyid
EOF

openssl ecparam -name prime256v1 -genkey -noout -out ca.key
openssl req -new -x509 -key ca.key -days 10 -config ca.cnf -extensions ext -out ca.pem

openssl ecparam -name prime256v1 -genkey -noout -out leaf.key
openssl req -new -key leaf.key -config leaf.cnf -out leaf.csr
openssl x509 -req -in leaf.csr -CA ca.pem -CAkey ca.key -CAcreateserial -days 10 \
    -extfile leaf.cnf -extensions ext -sha256 -out leaf.pem

openssl x509 -in leaf.pem -outform DER -out leaf.der
openssl pkcs8 -topk8 -nocrypt -in leaf.key -outform DER -out key.der
python3 -c 'import hashlib, sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' leaf.der >leaf.sha256

echo "leaf sha256: $(cat leaf.sha256)"
