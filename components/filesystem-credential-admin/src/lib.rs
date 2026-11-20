use exports::componentized::services::credential_admin::{Credential, Error, Guest, ServiceId};
use std::collections::HashMap;
use std::path::PathBuf;

mod fs;

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
    async fn publish(id: ServiceId, credentials: Vec<Credential>) -> Result<(), Error> {
        let mut creds = HashMap::new();
        for Credential { key, value } in credentials {
            creds.insert(key, value);
        }
        let creds = serde_json::to_string(&creds).map_err(|e| Error::from(e.to_string()))?;

        let path = FilesystemCredentialAdmin::get_path(id)?;
        let mut dir = path.clone();
        dir.pop();
        fs::create_dir_all(&dir.to_string_lossy()).await?;
        fs::write(&path.to_string_lossy(), creds.into_bytes()).await
    }

    async fn destroy(id: ServiceId) -> Result<(), Error> {
        fs::remove_file(&FilesystemCredentialAdmin::get_path(id)?.to_string_lossy()).await
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "filesystem-credential-admin",
    features: ["clocks-timezone"],
    generate_all
});

export!(FilesystemCredentialAdmin);
