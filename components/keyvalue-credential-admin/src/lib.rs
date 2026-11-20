use exports::componentized::services::credential_admin::{Credential, Error, Guest, ServiceId};
use serde_json;
use std::collections::HashMap;
use wasmcloud::keyvalue::store::{open, Bucket};

const BUCKET_KEY: &str = "bucket";
const BUCKET_DEFAULT: &str = "bindings";

pub(crate) struct KeyvalueCredentialAdmin;

impl KeyvalueCredentialAdmin {
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
            wasmcloud::keyvalue::types::Error::Other(msg) => {
                Error::from(format!("keyvalue: {msg}"))
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

impl Guest for KeyvalueCredentialAdmin {
    async fn publish(id: ServiceId, credentials: Vec<Credential>) -> Result<(), Error> {
        let mut creds = HashMap::new();
        for Credential { key, value } in credentials {
            creds.insert(key, value);
        }
        let creds = serde_json::to_vec(&creds).map_err(Self::map_serde_err)?;

        Self::get_bucket()
            .await?
            .set(id, creds, None)
            .await
            .map_err(Self::map_keyvalue_err)
    }

    async fn destroy(id: ServiceId) -> Result<(), Error> {
        Self::get_bucket()
            .await?
            .delete(id)
            .await
            .map_err(Self::map_keyvalue_err)
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "keyvalue-credential-admin",
    features: ["clocks-timezone"],
    generate_all
});

export!(KeyvalueCredentialAdmin);
