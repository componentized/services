use componentized::secrets::store::{self as secrets, Secret};
use exports::componentized::services_test_components::ops::{Error, Guest};
struct Ops;

impl Ops {
    async fn get_type() -> Result<OpsType, Error> {
        secrets::init()
            .await
            .map_err(|e| Error::from(e.to_string()))?;

        let t = secrets::get("type").map_err(|e| Error::from(e.to_string()))?;
        match t {
            Secret::String(type_) => match type_.as_str() {
                "filesystem" => Ok(OpsType::Filesystem),
                "valkey" => Ok(OpsType::Keyvalue),
                t => Err(Error::from(format!("Unknown client type: {t}"))),
            },
            Secret::Bytes(_) => Err(Error::from("Unknown client type")),
        }
    }
}

enum OpsType {
    Filesystem,
    Keyvalue,
}

impl Guest for Ops {
    async fn list(path: String) -> Result<Vec<String>, Error> {
        match Ops::get_type().await? {
            OpsType::Filesystem => filesystem_ops::list(path).await,
            OpsType::Keyvalue => keyvalue_ops::list(path).await,
        }
    }

    async fn read(path: String) -> Result<Vec<u8>, Error> {
        match Ops::get_type().await? {
            OpsType::Filesystem => filesystem_ops::read(path).await,
            OpsType::Keyvalue => keyvalue_ops::read(path).await,
        }
    }

    async fn write(path: String, data: Vec<u8>) -> Result<(), Error> {
        match Ops::get_type().await? {
            OpsType::Filesystem => filesystem_ops::write(path, data).await,
            OpsType::Keyvalue => keyvalue_ops::write(path, data).await,
        }
    }

    async fn move_(from_path: String, to_path: String) -> Result<(), Error> {
        match Ops::get_type().await? {
            OpsType::Filesystem => filesystem_ops::move_(from_path, to_path).await,
            OpsType::Keyvalue => keyvalue_ops::move_(from_path, to_path).await,
        }
    }

    async fn delete(path: String) -> Result<(), Error> {
        match Ops::get_type().await? {
            OpsType::Filesystem => filesystem_ops::delete(path).await,
            OpsType::Keyvalue => keyvalue_ops::delete(path).await,
        }
    }
}

wit_bindgen::generate!({
    path: "../../tests/wit",
    world: "ops-router",
    generate_all
});

export!(Ops);
