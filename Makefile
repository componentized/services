# the Valkey server of `make test-valkey`, a container removed afterwards, e.g. `VALKEY_VERSION=8.1`.
# The valkey client, componentized/valkey 0.2.2, fails to decode the HELLO reply of Valkey 9.
VALKEY_IMAGE ?= valkey/valkey
VALKEY_VERSION ?= 8

include build/build.mk

test-integration: test-filesystem test-valkey

# the test components built with cargo generate their bindings from tests/wit
$(foreach component,$(patsubst components/%/Cargo.toml,%,$(wildcard components/test-*/Cargo.toml)),${COMPONENTS_DIR}/$(component)/$(component).wasm ${COMPONENTS_DIR}/$(component)/$(component).debug.wasm): tests/wit/deps

# runs the services cli with the given arguments, in a shell where $$binding_id is the binding the
# ops use and $$config the config of the service, e.g. `$(call services,provision --type valkey)`.
# The services, their bindings and credentials are kept in tests/testdata, the root of the filesystem.
services = $(WASMTIME) run -Sconfig -Sinherit-network \
	-Sconfig-var=path=services \
	-Sconfig-var=binding-id="$${binding_id}" \
	$${config} \
	--dir tests/testdata::/ \
	${COMPONENTS_DIR}/test-cli/test-cli.wasm \
	$(1)

.PHONY: test-filesystem ## Run the services cli through the lifecycle of a filesystem service
test-filesystem: ${COMPONENTS_DIR}/test-cli/test-cli.wasm | $(call tool,wasmtime-cli)
	@set -e; \
	config="--env log_context_kv2fs"; \
	instance_id=$$($(call services,provision --type filesystem)); \
	$(call services,list-bindings $${instance_id}); \
	binding_id=$$($(call services,bind $${instance_id})); \
	$(call services,list-bindings $${instance_id}); \
	$(call services,credentials fetch $${binding_id}); \
	sleep 3; \
	$(call services,ops write foo 'Hello'); \
	$(call services,ops list /); \
	sleep 1; \
	$(call services,ops move foo bar); \
	$(call services,ops list /); \
	sleep 1; \
	$(call services,ops read bar); \
	sleep 1; \
	$(call services,ops delete bar); \
	$(call services,unbind $${binding_id} $${instance_id}); \
	$(call services,list-bindings $${instance_id}); \
	$(call services,destroy $${instance_id} --retain false)

# the server runs in a new container, on a free port, so it starts empty
.PHONY: test-valkey ## Run the services cli through the lifecycle of a valkey service, against a Valkey server in a container
test-valkey: ${COMPONENTS_DIR}/test-cli/test-cli.wasm | $(call tool,wasmtime-cli)
	@set -e; \
	container=$$(docker run --detach --rm --publish 127.0.0.1::6379 $(VALKEY_IMAGE):$(VALKEY_VERSION)); \
	trap 'docker stop $$container > /dev/null' EXIT; \
	port=$$(docker port $$container 6379/tcp | head -n 1 | sed 's/.*://'); \
	until docker exec $$container valkey-cli ping > /dev/null 2>&1; do sleep 0.1; done; \
	echo "Valkey $$(docker exec $$container valkey-server --version | sed 's/.* v=\([^ ]*\).*/\1/') on 127.0.0.1:$$port"; \
	config="-Sconfig-var=hostname=127.0.0.1 -Sconfig-var=port=$$port"; \
	instance_id=$$($(call services,provision --type valkey)); \
	read_write_binding_id=$$($(call services,bind $${instance_id})); \
	$(call services,credentials fetch $${read_write_binding_id}); \
	binding_id=$${read_write_binding_id}; \
	$(call services,ops write greeting 'Hello World!'); \
	$(call services,ops read greeting); \
	read_only_binding_id=$$($(call services,bind $${instance_id} --scopes read)); \
	$(call services,credentials fetch $${read_only_binding_id}); \
	binding_id=$${read_only_binding_id}; \
	$(call services,ops read greeting); \
	if $(call services,ops write greeting 'Uh oh!'); then echo "expected the write with a read only binding to fail" >&2; exit 1; fi; \
	$(call services,ops read greeting); \
	$(call services,unbind $${read_write_binding_id} $${instance_id}); \
	$(call services,unbind $${read_only_binding_id} $${instance_id}); \
	$(call services,destroy $${instance_id})
