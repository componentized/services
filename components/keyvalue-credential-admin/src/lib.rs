#![no_main]

use exports::componentized::services::credential_admin::{
    Credential, Error, Guest, ServiceBindingId,
};
use serde_json;
use std::collections::HashMap;
use wasi::keyvalue::store::{open, Bucket};

const BUCKET_KEY: &str = "bucket";
const BUCKET_DEFAULT: &str = "bindings";

pub(crate) struct KeyvalueCredentialAdmin;

impl KeyvalueCredentialAdmin {
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

impl Guest for KeyvalueCredentialAdmin {
    fn publish(binding_id: ServiceBindingId, credentials: Vec<Credential>) -> Result<(), Error> {
        let mut creds = HashMap::new();
        for Credential { key, value } in credentials {
            creds.insert(key, value);
        }
        let creds = serde_json::to_vec(&creds).map_err(Self::map_serde_err)?;

        Self::get_bucket()?
            .set(&binding_id, &creds)
            .map_err(Self::map_keyvalue_err)
    }

    fn destroy(binding_id: ServiceBindingId) -> Result<(), Error> {
        Self::get_bucket()?
            .delete(&binding_id)
            .map_err(Self::map_keyvalue_err)
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "keyvalue-credential-admin",
    generate_all
});

export!(KeyvalueCredentialAdmin);
