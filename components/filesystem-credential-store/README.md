# filesystem-credential-store

Implements `componentized:services/credential-store`, fetching the credentials of service bindings from files in `wasi:filesystem`, under the directory of the `path` key in `wasi:config`, `services` by default.
