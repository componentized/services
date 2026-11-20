# test-filesystem-chroot-config

Exports `wasi:config/store` with the `path` secret of the imported `componentized:secrets/store`, the config of a filesystem chroot client, e.g. [`dep-componentized-filesystem-chroot`](../dep-componentized-filesystem-chroot). No other secret is exposed. The `wasi:config/store` methods are sync, so the secrets store must be initialized elsewhere before the config is read, `init` is never called.
