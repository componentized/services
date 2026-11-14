# valkey-credential-admin

Implements `componentized:services/credential-admin`, publishing and destroying the credentials of service bindings in a Valkey server.

Plugs [`keyvalue-credential-admin`](../keyvalue-credential-admin/) with a [Valkey client](../dep-componentized-valkey-valkey-client/), at the `hostname` and `port` keys in `wasi:config`.
