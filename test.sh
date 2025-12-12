#!/bin/bash

set -o errexit
set -o pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
WASMTIME=${WASMTIME:-wasmtime}

store_type="${store_type:-filesystem}"
service_type="${service_type:-${store_type}}"

if [ -z ${SKIP_BUILD+x} ]; then
    ./build.sh "${cred_store_type:-${store_type}}"
fi

componentized_services() {
    ${WASMTIME} run -Sconfig -Sinherit-network \
        -Sconfig-var=path=services \
        --env log_context_kv2fs \
        -Sconfig-var=binding-id="${binding_id}" \
        --dir "${SCRIPT_DIR}/tests/testdata"::/ \
        "${SCRIPT_DIR}/lib/test/cli.wasm" \
        $@
}

instance_id=$(componentized_services provision --type "${service_type}")
componentized_services list-bindings ${instance_id}
binding_id=$(componentized_services bind ${instance_id})
componentized_services list-bindings ${instance_id}
componentized_services credentials fetch ${binding_id}
sleep 3
componentized_services ops write foo 'Hello'
componentized_services ops list /
sleep 1
componentized_services ops move foo bar
componentized_services ops list /
sleep 1
componentized_services ops read bar
sleep 1
componentized_services ops delete bar
componentized_services unbind ${binding_id} ${instance_id}
componentized_services list-bindings ${instance_id}
componentized_services destroy ${instance_id} --retain false
