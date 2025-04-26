#![no_main]

use exports::componentized::services::credential_admin::{
    Credential, Error, Guest, ServiceBindingId,
};
use wasi::logging::logging::{log, Level};

pub(crate) struct StubCredentialAdmin;

impl Guest for StubCredentialAdmin {
    fn publish(binding_id: ServiceBindingId, credentials: Vec<Credential>) -> Result<(), Error> {
        log(
            Level::Info,
            "credential-admin",
            &format!("publish binding-id={binding_id} credentials={credentials:?}"),
        );
        Ok(())
    }

    fn destroy(binding_id: ServiceBindingId) -> Result<(), Error> {
        log(
            Level::Info,
            "credential-admin",
            &format!("destroy binding-id={binding_id}"),
        );
        Ok(())
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "stub-credential-admin",
    generate_all
});

export!(StubCredentialAdmin);
