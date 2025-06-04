#![no_main]

use exports::componentized::services::credential_admin::{Credential, Error, Guest, ServiceId};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const PATH_KEY: &str = "path";
const PATH_DEFAULT: &str = "services";

pub(crate) struct FilesystemCredentialAdmin;

impl FilesystemCredentialAdmin {
    fn get_path(id: ServiceId) -> Result<PathBuf, Error> {
        let base_path = wasi::config::store::get(PATH_KEY)
            .map_err(|e| Error::from(e.to_string()))?
            .unwrap_or(String::from(PATH_DEFAULT));

        Ok(PathBuf::new().join(base_path).join("credentials").join(id))
    }
}

impl Guest for FilesystemCredentialAdmin {
    fn publish(id: ServiceId, credentials: Vec<Credential>) -> Result<(), Error> {
        let mut creds = HashMap::new();
        for Credential { key, value } in credentials {
            creds.insert(key, value);
        }
        let creds = serde_json::to_string(&creds).map_err(|e| Error::from(e.to_string()))?;

        let path = FilesystemCredentialAdmin::get_path(id)?;
        let mut dir = path.clone();
        dir.pop();
        fs::create_dir_all(dir).map_err(|e| Error::from(e.to_string()))?;
        fs::write(path, creds).map_err(|e| Error::from(e.to_string()))
    }

    fn destroy(id: ServiceId) -> Result<(), Error> {
        fs::remove_file(FilesystemCredentialAdmin::get_path(id)?)
            .map_err(|e| Error::from(e.to_string()))
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "filesystem-credential-admin",
    generate_all
});

export!(FilesystemCredentialAdmin);
