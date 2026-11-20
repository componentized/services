# valkey-lifecycle

Implements `componentized:services/service-lifecycle` for `valkey` services on a Valkey server, at the `hostname` and `port` keys in `wasi:config`, `127.0.0.1:6379` by default.

Plugs [`internal-valkey-lifecycle`](../internal-valkey-lifecycle/) with a [Valkey client](../dep-componentized-valkey-client/).
