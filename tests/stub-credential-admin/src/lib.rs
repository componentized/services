#![no_main]

use exports::componentized::services::credential_admin::{Credential, Error, Guest, ServiceId};
use wasi::logging::logging::{log, Level};

pub(crate) struct StubCredentialAdmin;

impl Guest for StubCredentialAdmin {
    fn publish(id: ServiceId, credentials: Vec<Credential>) -> Result<(), Error> {
        log(
            Level::Info,
            "credential-admin",
            &format!("publish id={id} credentials={credentials:?}"),
        );
        Ok(())
    }

    fn destroy(id: ServiceId) -> Result<(), Error> {
        log(Level::Info, "credential-admin", &format!("destroy id={id}"));
        Ok(())
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "stub-credential-admin",
    generate_all
});

export!(StubCredentialAdmin);
