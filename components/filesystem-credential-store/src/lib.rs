#![no_main]

use exports::componentized::services::credential_store::{
    Credential, Error, Guest, ServiceBindingId,
};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const PATH_KEY: &str = "path";
const PATH_DEFAULT: &str = "services";

pub(crate) struct FilesystemCredentialStore;

impl FilesystemCredentialStore {
    fn get_credential_path(binding_id: ServiceBindingId) -> Result<PathBuf, Error> {
        let base_path = wasi::config::store::get(PATH_KEY)
            .map_err(|e| Error::from(e.to_string()))?
            .unwrap_or(String::from(PATH_DEFAULT));

        Ok(PathBuf::new()
            .join(base_path)
            .join("credentials")
            .join(binding_id))
    }
}

impl Guest for FilesystemCredentialStore {
    fn fetch(id: ServiceBindingId) -> Result<Vec<Credential>, Error> {
        let bytes = fs::read(FilesystemCredentialStore::get_credential_path(id)?)
            .map_err(|e| Error::from(e.to_string()))?;
        let creds: HashMap<String, String> =
            serde_json::from_slice(bytes.as_slice()).map_err(|e| Error::from(e.to_string()))?;
        let creds = creds
            .iter()
            .map(|(k, v)| Credential {
                key: k.to_string(),
                value: v.to_string(),
            })
            .collect();
        Ok(creds)
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "filesystem-credential-store",
    generate_all
});

export!(FilesystemCredentialStore);
