#![no_main]

use exports::componentized::services::ops::{Error, Guest};
use std::fs;

struct FilesystemOps;

impl Guest for FilesystemOps {
    fn list(path: String) -> Result<Vec<String>, Error> {
        let dir = fs::read_dir(path).map_err(|e| Error::from(e.to_string()))?;
        let file_names = dir
            .map(|f| String::from(f.unwrap().file_name().to_str().unwrap()))
            .collect();
        Ok(file_names)
    }

    fn read(path: String) -> Result<Vec<u8>, Error> {
        fs::read(path).map_err(|e| Error::from(e.to_string()))
    }

    fn write(path: String, data: Vec<u8>) -> Result<(), Error> {
        fs::write(path, data).map_err(|e| Error::from(e.to_string()))
    }

    fn move_(from_path: String, to_path: String) -> Result<(), Error> {
        fs::rename(from_path, to_path).map_err(|e| Error::from(e.to_string()))
    }

    fn delete(path: String) -> Result<(), Error> {
        fs::remove_file(path).map_err(|e| Error::from(e.to_string()))
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "filesystem-ops",
    generate_all
});

export!(FilesystemOps);
