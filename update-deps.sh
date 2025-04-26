#!/bin/bash

set -o errexit
set -o pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

DEPS_DIR="${SCRIPT_DIR}/lib/deps"

rm -rf "${DEPS_DIR}"
mkdir -p "${DEPS_DIR}"

wkg oci pull ghcr.io/componentized/logging/levels:v0.1.0 -o "${DEPS_DIR}/logging-levels.wasm"
wkg oci pull ghcr.io/componentized/logging/to-stdout:v0.1.0 -o "${DEPS_DIR}/logging-to-stdout.wasm"
wac plug "${DEPS_DIR}/logging-levels.wasm" --plug "${DEPS_DIR}/logging-to-stdout.wasm" -o "${DEPS_DIR}/logger.wasm"
static-config -o "${DEPS_DIR}/app-config.wasm" -p logging.env.prefix=log_context_

wkg oci pull ghcr.io/componentized/filesystem/chroot:v0.1.0 -o "${DEPS_DIR}/filesystem-chroot.wasm"
wkg oci pull ghcr.io/componentized/cli/stdout-to-stderr:v0.1.0 -o "${DEPS_DIR}/stdout-to-stderr.wasm"
wkg oci pull ghcr.io/componentized/valkey/valkey-client:v0.1.1 -o "${DEPS_DIR}/valkey-client.wasm"
wkg oci pull ghcr.io/componentized/static-config/factory:v0.1.0 -o "${DEPS_DIR}/static-config-factory.wasm"

wkg wit fetch
(cd components && wkg wit fetch)
(cd tests && wkg wit fetch)
