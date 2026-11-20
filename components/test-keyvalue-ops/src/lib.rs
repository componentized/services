use exports::componentized::services_test_components::ops::{Error, Guest};
use wasmcloud::keyvalue::store;

struct KeyvalueOps;

impl KeyvalueOps {
    async fn open() -> Result<store::Bucket, Error> {
        store::open("ops".to_string()).await.map_err(map_err)
    }
}

impl Guest for KeyvalueOps {
    async fn list(path: String) -> Result<Vec<String>, Error> {
        let response = Self::open()
            .await?
            .list_keys(None, None)
            .await
            .map_err(map_err)?;

        let mut keys = vec![];
        for key in response.keys {
            if key.starts_with(&path) {
                keys.push(key);
            }
        }

        Ok(keys)
    }

    async fn read(path: String) -> Result<Vec<u8>, Error> {
        match Self::open()
            .await?
            .get(path.clone())
            .await
            .map_err(map_err)?
        {
            Some(value) => Ok(value),
            None => Err(format!("Path '{path}' not found")),
        }
    }

    async fn write(path: String, data: Vec<u8>) -> Result<(), Error> {
        Self::open()
            .await?
            .set(path, data, None)
            .await
            .map_err(map_err)
    }

    async fn move_(from_path: String, to_path: String) -> Result<(), Error> {
        let bucket = Self::open().await?;

        match bucket.get(from_path.clone()).await.map_err(map_err)? {
            Some(value) => {
                bucket.set(to_path, value, None).await.map_err(map_err)?;
                bucket.delete(from_path).await.map_err(map_err)?;

                Ok(())
            }
            None => Err(Error::from(format!("Path '{from_path}' not found"))),
        }
    }

    async fn delete(path: String) -> Result<(), Error> {
        Self::open().await?.delete(path).await.map_err(map_err)
    }
}

fn map_err(err: store::Error) -> Error {
    match err {
        store::Error::NoSuchStore => Error::from("keyvalue: no-such-store"),
        store::Error::AccessDenied => Error::from("keyvalue: access-denied"),
        store::Error::InvalidArgument => Error::from("keyvalue: invalid-argument"),
        store::Error::PreconditionFailed => Error::from("keyvalue: precondiion-failed"),
        store::Error::Timeout => Error::from("keyvalue: timeout"),
        store::Error::StoreUnavailable => Error::from("keyvalue: store-unavailable"),
        store::Error::QuotaExceeded => Error::from("keyvalue: quota-exceeded"),
        store::Error::Other(message) => Error::from(format!("keyvalue: {message}")),
    }
}

wit_bindgen::generate!({
    path: "../../tests/wit",
    world: "keyvalue-ops",
    generate_all
});

export!(KeyvalueOps);
