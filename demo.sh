#!/bin/bash

set -o errexit
set -o pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

componentized_services() {
    ${WASMTIME:-wasmtime} run -Sconfig -Sinherit-network \
        -Sconfig-var=path=services \
        -Sconfig-var=binding-id="${binding_id}" \
        --dir "${SCRIPT_DIR:-.}/tests/testdata"::/ \
        "${SCRIPT_DIR:-.}/lib/test/cli.wasm" \
        "$@"
}

# create valkey service, capturing the instance_id from stdout
instance_id=$(componentized_services provision --type valkey)

# create read/write binding, capturing the binding_id from stdout
read_write_binding_id=$(componentized_services bind ${instance_id})
componentized_services credentials fetch ${read_write_binding_id}

# write and read a value
binding_id=${read_write_binding_id} componentized_services ops write greeting 'Hello World!'
binding_id=${read_write_binding_id} componentized_services ops read greeting

# create read-only binding, capturing the binding_id from stdout
read_only_binding_id=$(componentized_services bind ${instance_id} --scopes read)
componentized_services credentials fetch ${read_only_binding_id}

# can read previous values, but not write
binding_id=${read_only_binding_id} componentized_services ops read greeting
binding_id=${read_only_binding_id} componentized_services ops write greeting 'Uh oh!' || echo 'Previous command expected to fail'

# verify the key was not modified
binding_id=${read_only_binding_id} componentized_services ops read greeting

# cleanup
componentized_services unbind ${read_write_binding_id} ${instance_id}
componentized_services unbind ${read_only_binding_id} ${instance_id}
componentized_services destroy ${instance_id}
