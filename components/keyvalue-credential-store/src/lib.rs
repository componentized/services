#![no_main]

use exports::componentized::services::credential_store::{
    Credential, Error, Guest, ServiceBindingId,
};
use serde_json;
use std::collections::HashMap;
use std::vec;
use wasi::keyvalue::store::{open, Bucket};

const BUCKET_KEY: &str = "bucket";
const BUCKET_DEFAULT: &str = "bindings";

pub(crate) struct KeyvalueCredentialStore;

impl KeyvalueCredentialStore {
    fn get_bucket() -> Result<Bucket, Error> {
        let bucket_id = wasi::config::store::get(BUCKET_KEY)
            .map_err(Self::map_config_err)?
            .unwrap_or(String::from(BUCKET_DEFAULT));
        open(&bucket_id).map_err(Self::map_keyvalue_err)
    }

    fn map_keyvalue_err(e: wasi::keyvalue::store::Error) -> Error {
        match e {
            wasi::keyvalue::store::Error::NoSuchStore => Error::from("NoSuchStore Error"),
            wasi::keyvalue::store::Error::AccessDenied => Error::from("AccessDenied Error"),
            wasi::keyvalue::store::Error::Other(msg) => Error::from(format!("Other Error: {msg}")),
        }
    }

    fn map_config_err(e: wasi::config::store::Error) -> Error {
        match e {
            wasi::config::store::Error::Upstream(msg) => {
                Error::from(format!("Upstream Error: {msg}"))
            }
            wasi::config::store::Error::Io(msg) => Error::from(format!("Io Error: {msg}")),
        }
    }

    fn map_serde_err(e: serde_json::Error) -> Error {
        Error::from(format!("JSON Error: {e}"))
    }
}

impl Guest for KeyvalueCredentialStore {
    fn fetch(binding_id: ServiceBindingId) -> Result<Vec<Credential>, Error> {
        let creds = Self::get_bucket()?
            .get(&binding_id)
            .map_err(Self::map_keyvalue_err)?
            .unwrap_or(vec![]);

        let creds: HashMap<String, String> =
            serde_json::from_slice(&creds.as_slice()).map_err(Self::map_serde_err)?;
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
    world: "keyvalue-credential-store",
    generate_all
});

export!(KeyvalueCredentialStore);
