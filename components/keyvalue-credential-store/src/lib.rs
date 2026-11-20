use exports::componentized::services::credential_store::{Credential, Error, Guest, ServiceId};
use serde_json;
use std::collections::HashMap;
use std::vec;
use wasmcloud::keyvalue::store::{open, Bucket};

const BUCKET_KEY: &str = "bucket";
const BUCKET_DEFAULT: &str = "bindings";

pub(crate) struct KeyvalueCredentialStore;

impl KeyvalueCredentialStore {
    async fn get_bucket() -> Result<Bucket, Error> {
        let bucket_id = wasi::config::store::get(BUCKET_KEY)
            .map_err(Self::map_config_err)?
            .unwrap_or(String::from(BUCKET_DEFAULT));
        open(bucket_id).await.map_err(Self::map_keyvalue_err)
    }

    fn map_keyvalue_err(e: wasmcloud::keyvalue::store::Error) -> Error {
        match e {
            wasmcloud::keyvalue::types::Error::NoSuchStore => {
                Error::from("keyvalue: no-such-store")
            }
            wasmcloud::keyvalue::types::Error::AccessDenied => {
                Error::from("keyvalue: access-denied")
            }
            wasmcloud::keyvalue::types::Error::InvalidArgument => {
                Error::from("keyvalue: invalid-argument")
            }
            wasmcloud::keyvalue::types::Error::PreconditionFailed => {
                Error::from("keyvalue: precondition-failed")
            }
            wasmcloud::keyvalue::types::Error::Timeout => Error::from("keyvalue: timeout"),
            wasmcloud::keyvalue::types::Error::StoreUnavailable => {
                Error::from("keyvalue: store-unavailable")
            }
            wasmcloud::keyvalue::types::Error::QuotaExceeded => {
                Error::from("keyvalue: quota-exceeded")
            }
            wasmcloud::keyvalue::types::Error::Other(message) => {
                Error::from(format!("keyvalue: {message}"))
            }
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
    async fn fetch(id: ServiceId) -> Result<Vec<Credential>, Error> {
        let creds = Self::get_bucket()
            .await?
            .get(id)
            .await
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
    features: ["clocks-timezone"],
    generate_all
});

export!(KeyvalueCredentialStore);
