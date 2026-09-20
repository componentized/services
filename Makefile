.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: publish ## Publish each component in the lib directory
publish: $(shell find lib -maxdepth 1 -type f -name "*.wasm" | sed -e 's:^lib/:publish-:g')

.PHONY: publish-%
publish-%:
ifndef VERSION
	$(error VERSION is undefined)
endif
ifndef REPOSITORY
	$(error REPOSITORY is undefined)
endif
	@$(eval FILE := $(@:publish-%=%))
	@$(eval COMPONENT := $(if $(filter %.debug.wasm,$(FILE)),$(FILE:%.debug.wasm=%),$(FILE:%.wasm=%)))
	@$(eval TITLE := $(if $(filter %.debug.wasm,$(FILE)),$(COMPONENT) (debug),$(COMPONENT)))
# 	@$(eval DESCRIPTION := $(shell head -n 3 "lib/${FILE}.md" | tail -n 1))
	@$(eval REVISION := $(shell git rev-parse HEAD)$(shell git diff --quiet HEAD && echo "+dirty"))
	@$(eval COMPONENT_VERSION := $(if $(filter %.debug.wasm,$(FILE)),${VERSION}+debug,${VERSION}))
	@$(eval TAG := $(patsubst v%,%,$(subst +,_,$(COMPONENT_VERSION))))
	@$(eval IMAGE := $(if $(filter interface.wasm,$(FILE)),${REPOSITORY}:${TAG},${REPOSITORY}/${COMPONENT}:${TAG}))

# 			--annotation "org.opencontainers.image.description=${DESCRIPTION}" \

	@echo "::group::${FILE} -> ${IMAGE}"
	@DIGEST=$$( \
		wkg oci push \
			--annotation "org.opencontainers.image.title=${TITLE}" \
			--annotation "org.opencontainers.image.version=${COMPONENT_VERSION}" \
			--annotation "org.opencontainers.image.source=https://github.com/${GITHUB_REPOSITORY}.git" \
			--annotation "org.opencontainers.image.revision=${REVISION}" \
			--annotation "org.opencontainers.image.licenses=Apache-2.0" \
			"${IMAGE}" \
			"lib/${FILE}" \
			2>&1 \
			| tee /dev/stderr \
			| grep -o 'sha256:[a-f0-9]\{64\}' \
	) ; \
	cosign sign --yes "${IMAGE}@$${DIGEST}"
	@echo "::endgroup::"
