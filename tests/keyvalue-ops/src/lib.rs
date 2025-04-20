#![no_main]

use exports::componentized::services::ops::{Error, Guest};
use wasi::keyvalue::store;

struct KeyvalueOps;

impl KeyvalueOps {
    fn open() -> Result<store::Bucket, Error> {
        store::open("ops").map_err(map_err)
    }
}

impl Guest for KeyvalueOps {
    fn list(path: String) -> Result<Vec<String>, Error> {
        let response = Self::open()?.list_keys(None).map_err(map_err)?;

        let mut keys = vec![];
        for key in response.keys {
            if key.starts_with(&path) {
                keys.push(key);
            }
        }

        Ok(keys)
    }

    fn read(path: String) -> Result<Vec<u8>, Error> {
        match Self::open()?.get(&path).map_err(map_err)? {
            Some(value) => Ok(value),
            None => Err(format!("Path '{path}' not found")),
        }
    }

    fn write(path: String, data: Vec<u8>) -> Result<(), Error> {
        Self::open()?.set(&path, &data).map_err(map_err)
    }

    fn move_(from_path: String, to_path: String) -> Result<(), Error> {
        let bucket = Self::open()?;

        match bucket.get(&from_path).map_err(map_err)? {
            Some(value) => {
                bucket.set(&to_path, &value).map_err(map_err)?;
                bucket.delete(&from_path).map_err(map_err)?;

                Ok(())
            }
            None => Err(Error::from(format!("Path '{from_path}' not found"))),
        }
    }

    fn delete(path: String) -> Result<(), Error> {
        Self::open()?.delete(&path).map_err(map_err)
    }
}

fn map_err(err: store::Error) -> Error {
    match err {
        store::Error::NoSuchStore => Error::from("No such store"),
        store::Error::AccessDenied => Error::from("Access denied"),
        store::Error::Other(msg) => Error::from(msg),
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "keyvalue-ops",
    generate_all
});

export!(KeyvalueOps);
