#!/bin/bash

set -o errexit
set -o pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

cred_store_type="${1:-filesystem}"

rm -rf "${SCRIPT_DIR}"/lib/*.wasm
rm -rf "${SCRIPT_DIR}/lib/test" && mkdir -p "${SCRIPT_DIR}/lib/test"

# wit interface

wkg wit build -o "${SCRIPT_DIR}/lib/interface.wasm"

# core components

cargo component build -p credential-config --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/credential_config.wasm" "${SCRIPT_DIR}/lib/credential-config.wasm"
cargo component build -p keyvalue-credential-admin --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/keyvalue_credential_admin.wasm" "${SCRIPT_DIR}/lib/keyvalue-credential-admin.wasm"
cargo component build -p keyvalue-credential-store --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/keyvalue_credential_store.wasm" "${SCRIPT_DIR}/lib/keyvalue-credential-store.wasm"
cargo component build -p lifecycle-host-cli --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/lifecycle_host_cli.wasm" "${SCRIPT_DIR}/lib/lifecycle-host-cli.wasm"
cargo component build -p lifecycle-host-http --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/lifecycle_host_http.wasm" "${SCRIPT_DIR}/lib/lifecycle-host-http.wasm"

# filesystem components

cargo component build -p filesystem-lifecycle --release --target wasm32-wasip2
cp "${SCRIPT_DIR}/target/wasm32-wasip2/release/filesystem_lifecycle.wasm" "${SCRIPT_DIR}/lib/filesystem-lifecycle.wasm"
cargo component build -p filesystem-credential-store --release --target wasm32-wasip2
cp "${SCRIPT_DIR}/target/wasm32-wasip2/release/filesystem_credential_store.wasm" "${SCRIPT_DIR}/lib/filesystem-credential-store.wasm"
cargo component build -p filesystem-credential-admin --release --target wasm32-wasip2
cp "${SCRIPT_DIR}/target/wasm32-wasip2/release/filesystem_credential_admin.wasm" "${SCRIPT_DIR}/lib/filesystem-credential-admin.wasm"


# valkey components

cargo component build -p valkey-lifecycle --release --target wasm32-wasip2
wac plug "${SCRIPT_DIR}/target/wasm32-wasip2/release/valkey_lifecycle.wasm" --plug "${SCRIPT_DIR}/lib/deps/valkey-client.wasm" -o "${SCRIPT_DIR}/lib/valkey-lifecycle.wasm"
wac plug "${SCRIPT_DIR}/lib/keyvalue-credential-store.wasm" --plug "${SCRIPT_DIR}/lib/deps/valkey-client.wasm" -o "${SCRIPT_DIR}/lib/valkey-credential-store.wasm"
wac plug "${SCRIPT_DIR}/lib/keyvalue-credential-admin.wasm" --plug "${SCRIPT_DIR}/lib/deps/valkey-client.wasm" -o "${SCRIPT_DIR}/lib/valkey-credential-admin.wasm"


# webhook components

cargo component build -p webhook-credential-admin --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/webhook_credential_admin.wasm" "${SCRIPT_DIR}/lib/webhook-credential-admin.wasm"
cargo component build -p webhook-credential-store --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/webhook_credential_store.wasm" "${SCRIPT_DIR}/lib/webhook-credential-store.wasm"

# test components
cp "${SCRIPT_DIR}/lib/deps/filesystem-chroot.wasm" "${SCRIPT_DIR}/lib/test/filesystem-client.wasm"
cp "${SCRIPT_DIR}/lib/deps/valkey-client.wasm" "${SCRIPT_DIR}/lib/test/keyvalue-client.wasm"
cargo component build -p filesystem-ops --release --target wasm32-wasip2
cp "${SCRIPT_DIR}/target/wasm32-wasip2/release/filesystem_ops.wasm" "${SCRIPT_DIR}/lib/test/filesystem-ops.wasm"
cargo component build -p keyvalue-ops --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/keyvalue_ops.wasm" "${SCRIPT_DIR}/lib/test/keyvalue-ops.wasm"
cargo component build -p lifecycle-router --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/lifecycle_router.wasm" "${SCRIPT_DIR}/lib/test/lifecycle-router.wasm"
cargo component build -p ops-router --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/ops_router.wasm" "${SCRIPT_DIR}/lib/test/ops-router.wasm"
cargo component build -p service-cli --release --target wasm32-wasip2
cp "${SCRIPT_DIR}/target/wasm32-wasip2/release/service-cli.wasm" "${SCRIPT_DIR}/lib/test/service-cli.wasm"

# stub components

cargo component build -p stub-lifecycle --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/stub_lifecycle.wasm" "${SCRIPT_DIR}/lib/test/stub-lifecycle.wasm"
cargo component build -p stub-client --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/stub_client.wasm" "${SCRIPT_DIR}/lib/test/stub-client.wasm"
cargo component build -p stub-credential-admin --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/stub_credential_admin.wasm" "${SCRIPT_DIR}/lib/test/stub-credential-admin.wasm"
cargo component build -p stub-credential-store --release --target wasm32-unknown-unknown
cp "${SCRIPT_DIR}/target/wasm32-unknown-unknown/release/stub_credential_store.wasm" "${SCRIPT_DIR}/lib/test/stub-credential-store.wasm"

wac compose -o "${SCRIPT_DIR}/lib/test/logging.wasm" \
    -d componentized:logger="${SCRIPT_DIR}/lib/deps/logger.wasm" \
    -d componentized:app-config="${SCRIPT_DIR}/lib/deps/app-config.wasm" \
    -d componentized:stdout-to-stderr="${SCRIPT_DIR}/lib/deps/stdout-to-stderr.wasm" \
    "${SCRIPT_DIR}/tests/logging.wac"

wac compose -o "${SCRIPT_DIR}/lib/test/lifecycle.wasm" \
    -d componentized:logging="${SCRIPT_DIR}/lib/logging.wasm" \
    -d componentized:lifecycle-router="${SCRIPT_DIR}/lib/test/lifecycle-router.wasm" \
    -d componentized:filesystem-lifecycle="${SCRIPT_DIR}/lib/filesystem-lifecycle.wasm" \
    -d componentized:keyvalue-lifecycle="${SCRIPT_DIR}/lib/valkey-lifecycle.wasm" \
    "${SCRIPT_DIR}/tests/lifecycle.wac"

wac compose -o "${SCRIPT_DIR}/lib/test/ops.wasm" \
    -d componentized:logging="${SCRIPT_DIR}/lib/logging.wasm" \
    -d componentized:credential-store="${SCRIPT_DIR}/lib/${cred_store_type}-credential-store.wasm" \
    -d componentized:credential-config="${SCRIPT_DIR}/lib/credential-config.wasm" \
    -d componentized:filesystem-client="${SCRIPT_DIR}/lib/test/filesystem-client.wasm" \
    -d componentized:filesystem-ops="${SCRIPT_DIR}/lib/test/filesystem-ops.wasm" \
    -d componentized:keyvalue-client="${SCRIPT_DIR}/lib/test/keyvalue-client.wasm" \
    -d componentized:keyvalue-ops="${SCRIPT_DIR}/lib/test/keyvalue-ops.wasm" \
    -d componentized:ops-router="${SCRIPT_DIR}/lib/test/ops-router.wasm" \
    "${SCRIPT_DIR}/tests/ops.wac"

wac compose -o "${SCRIPT_DIR}/lib/test/cli.wasm" \
    -d componentized:logging="${SCRIPT_DIR}/lib/logging.wasm" \
    -d componentized:cli="${SCRIPT_DIR}/lib/test/service-cli.wasm" \
    -d componentized:lifecycle="${SCRIPT_DIR}/lib/test/lifecycle.wasm" \
    -d componentized:credential-store="${SCRIPT_DIR}/lib/${cred_store_type}-credential-store.wasm" \
    -d componentized:credential-admin="${SCRIPT_DIR}/lib/${cred_store_type}-credential-admin.wasm" \
    -d componentized:ops="${SCRIPT_DIR}/lib/test/ops.wasm" \
    -d componentized:static-config-factory="${SCRIPT_DIR}/lib/deps/static-config-factory.wasm" \
    "${SCRIPT_DIR}/tests/cli.wac"

wac compose -o "${SCRIPT_DIR}/lib/test/host-cli-valkey.wasm" \
    -d componentized:logging="${SCRIPT_DIR}/lib/test/logging.wasm" \
    -d componentized:lifecycle-host="${SCRIPT_DIR}/lib/lifecycle-host-cli.wasm" \
    -d componentized:lifecycle="${SCRIPT_DIR}/lib/valkey-lifecycle.wasm" \
    -d componentized:credential-admin="${SCRIPT_DIR}/lib/valkey-credential-admin.wasm" \
    "${SCRIPT_DIR}/tests/host.wac"

wac compose -o "${SCRIPT_DIR}/lib/test/host-http-valkey.wasm" \
    -d componentized:logging="${SCRIPT_DIR}/lib/test/logging.wasm" \
    -d componentized:lifecycle-host="${SCRIPT_DIR}/lib/lifecycle-host-http.wasm" \
    -d componentized:lifecycle="${SCRIPT_DIR}/lib/valkey-lifecycle.wasm" \
    -d componentized:credential-admin="${SCRIPT_DIR}/lib/valkey-credential-admin.wasm" \
    "${SCRIPT_DIR}/tests/host.wac"
