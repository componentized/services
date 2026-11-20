# valkey-credential-store

Implements `componentized:services/credential-store`, fetching the credentials of service bindings from a Valkey server.

Plugs [`keyvalue-credential-store`](../keyvalue-credential-store/) with a [Valkey client](../dep-componentized-valkey-client/), at the `hostname` and `port` keys in `wasi:config`.
