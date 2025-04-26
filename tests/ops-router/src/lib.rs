#![no_main]

use exports::componentized::services_test_components::ops::{Error, Guest};

struct Ops;

impl Ops {
    fn get_type() -> Result<OpsType, Error> {
        let t = wasi::config::store::get("type").map_err(|e| Error::from(e.to_string()))?;
        match t.as_deref() {
            Some("filesystem") => Ok(OpsType::Filesystem),
            Some("valkey") => Ok(OpsType::Keyvalue),
            Some(t) => Err(Error::from(format!("Unknown client type {t}"))),
            None => Err(Error::from(format!("Unknown client type"))),
        }
    }
}

enum OpsType {
    Filesystem,
    Keyvalue,
}

impl Guest for Ops {
    fn list(path: String) -> Result<Vec<String>, Error> {
        match Ops::get_type()? {
            OpsType::Filesystem => filesystem_ops::list(&path),
            OpsType::Keyvalue => keyvalue_ops::list(&path),
        }
    }

    fn read(path: String) -> Result<Vec<u8>, Error> {
        match Ops::get_type()? {
            OpsType::Filesystem => filesystem_ops::read(&path),
            OpsType::Keyvalue => keyvalue_ops::read(&path),
        }
    }

    fn write(path: String, data: Vec<u8>) -> Result<(), Error> {
        match Ops::get_type()? {
            OpsType::Filesystem => filesystem_ops::write(&path, &data),
            OpsType::Keyvalue => keyvalue_ops::write(&path, &data),
        }
    }

    fn move_(from_path: String, to_path: String) -> Result<(), Error> {
        match Ops::get_type()? {
            OpsType::Filesystem => filesystem_ops::move_(&from_path, &to_path),
            OpsType::Keyvalue => keyvalue_ops::move_(&from_path, &to_path),
        }
    }

    fn delete(path: String) -> Result<(), Error> {
        match Ops::get_type()? {
            OpsType::Filesystem => filesystem_ops::delete(&path),
            OpsType::Keyvalue => keyvalue_ops::delete(&path),
        }
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "ops-router",
    generate_all
});

export!(Ops);
