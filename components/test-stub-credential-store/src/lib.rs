use exports::componentized::services::credential_store::{Credential, Error, Guest, ServiceId};
use wasi::logging::logging::{log, Level};

pub(crate) struct StubCredentialStore;

impl Guest for StubCredentialStore {
    async fn fetch(id: ServiceId) -> Result<Vec<Credential>, Error> {
        log(Level::Info, "credential-store", &format!("fetch id={id}"));
        Ok(vec![])
    }
}

wit_bindgen::generate!({
    path: "../../tests/wit",
    world: "stub-credential-store",
    generate_all
});

export!(StubCredentialStore);
