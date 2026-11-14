# keyvalue-credential-store

Implements `componentized:services/credential-store`, fetching the credentials of service bindings from a `wasi:keyvalue` bucket, named by the `bucket` key in `wasi:config`, `bindings` by default.
