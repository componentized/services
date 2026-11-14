# test-ops

Composes the data operations of each service type, [`test-filesystem-ops`](../test-filesystem-ops/) over [`chroot`](../dep-componentized-filesystem-chroot/) and [`test-keyvalue-ops`](../test-keyvalue-ops/) over a [Valkey client](../dep-componentized-valkey-valkey-client/), each configured with the credentials of a binding by [`credential-config`](../credential-config/), behind [`test-ops-router`](../test-ops-router/).
