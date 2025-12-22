#![no_main]

use componentized::services::credential_store::fetch;
use exports::wasi::config::store::{Error, Guest};
use wasi::config::store;

const BINDING_ID_KEY: &str = "binding-id";

pub(crate) struct CredentialConfig {}

impl Guest for CredentialConfig {
    fn get(key: String) -> Result<Option<String>, Error> {
        Ok(CredentialConfig::get_all()?.iter().find_map(|(k, v)| {
            if key == *k {
                Some(v.clone())
            } else {
                None
            }
        }))
    }

    fn get_all() -> Result<Vec<(String, String)>, Error> {
        match store::get(BINDING_ID_KEY).map_err(config_err_map)? {
            None => Err(Error::Upstream(String::from(format!(
                "Config must contain '{}'",
                BINDING_ID_KEY
            )))),
            Some(binding_id) => {
                let credentials = fetch(&binding_id)
                    .map_err(|e| Error::Upstream(e.to_string()))?
                    .iter()
                    .map(|c| (c.key.clone(), c.value.clone()))
                    .collect();

                Ok(credentials)
            }
        }
    }
}

fn config_err_map(err: store::Error) -> Error {
    match err {
        store::Error::Upstream(err) => Error::Upstream(err),
        store::Error::Io(err) => Error::Io(err),
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "credential-config",
    features: ["clocks-timezone"],
    generate_all
});

export!(CredentialConfig);
