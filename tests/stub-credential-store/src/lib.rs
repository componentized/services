#![no_main]

use exports::componentized::services::credential_store::{
    Credential, Error, Guest, ServiceBindingId,
};
use wasi::logging::logging::{log, Level};

pub(crate) struct StubCredentialStore;

impl Guest for StubCredentialStore {
    fn fetch(binding_id: ServiceBindingId) -> Result<Vec<Credential>, Error> {
        log(
            Level::Info,
            "credential-store",
            &format!("fetch binding-id={binding_id}"),
        );
        Ok(vec![])
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "stub-credential-store",
    generate_all
});

export!(StubCredentialStore);
