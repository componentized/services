.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: publish ## Publish each component in the lib directory
publish: $(shell find lib -type f -name "*.wasm" | sed -e 's:^lib/:publish-:g')

.PHONY: publish-%
publish-%:
ifndef VERSION
	$(error VERSION is undefined)
endif
ifndef REPOSITORY
	$(error REPOSITORY is undefined)
endif
	@$(eval FILE := $(@:publish-%=%))
	@$(eval COMPONENT := $(FILE:%.wasm=%))
# 	@$(eval DESCRIPTION := $(shell head -n 3 "lib/${FILE}.md" | tail -n 1))
	@$(eval REVISION := $(shell git rev-parse HEAD)$(shell git diff --quiet HEAD && echo "+dirty"))
	@$(eval TAG := $(shell echo "${VERSION}" | sed 's/[^a-zA-Z0-9_.\-]/--/g'))

# 			--annotation "org.opencontainers.image.description=${DESCRIPTION}" \

	@echo "::group::${FILE} -> ${REPOSITORY}/${COMPONENT}:${TAG}"
	@DIGEST=$$( \
		wkg oci push \
			--annotation "org.opencontainers.image.title=${COMPONENT}" \
			--annotation "org.opencontainers.image.version=${VERSION}" \
			--annotation "org.opencontainers.image.source=https://github.com/${GITHUB_REPOSITORY}.git" \
			--annotation "org.opencontainers.image.revision=${REVISION}" \
			--annotation "org.opencontainers.image.licenses=Apache-2.0" \
			"${REPOSITORY}/${COMPONENT}:${TAG}" \
			"lib/${FILE}" \
			2>&1 \
			| tee /dev/stderr \
			| grep -o 'sha256:[a-f0-9]\{64\}' \
	) ; \
	cosign sign --yes "${REPOSITORY}/${COMPONENT}:${TAG}@$${DIGEST}"
	@echo "::endgroup::"
