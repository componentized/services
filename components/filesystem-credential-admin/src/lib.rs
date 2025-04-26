#![no_main]

use exports::componentized::services::credential_admin::{
    Credential, Error, Guest, ServiceBindingId,
};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const PATH_KEY: &str = "path";
const PATH_DEFAULT: &str = "services";

pub(crate) struct FilesystemCredentialAdmin;

impl FilesystemCredentialAdmin {
    fn get_binding_path(binding_id: ServiceBindingId) -> Result<PathBuf, Error> {
        let base_path = wasi::config::store::get(PATH_KEY)
            .map_err(|e| Error::from(e.to_string()))?
            .unwrap_or(String::from(PATH_DEFAULT));

        Ok(PathBuf::new()
            .join(base_path)
            .join("credentials")
            .join(binding_id))
    }
}

impl Guest for FilesystemCredentialAdmin {
    fn publish(binding_id: ServiceBindingId, credentials: Vec<Credential>) -> Result<(), Error> {
        let mut creds = HashMap::new();
        for Credential { key, value } in credentials {
            creds.insert(key, value);
        }
        let creds = serde_json::to_string(&creds).map_err(|e| Error::from(e.to_string()))?;

        let path = FilesystemCredentialAdmin::get_binding_path(binding_id)?;
        let mut dir = path.clone();
        dir.pop();
        fs::create_dir_all(dir).map_err(|e| Error::from(e.to_string()))?;
        fs::write(path, creds).map_err(|e| Error::from(e.to_string()))
    }

    fn destroy(id: ServiceBindingId) -> Result<(), Error> {
        fs::remove_file(FilesystemCredentialAdmin::get_binding_path(id)?)
            .map_err(|e| Error::from(e.to_string()))
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "filesystem-credential-admin",
    generate_all
});

export!(FilesystemCredentialAdmin);
