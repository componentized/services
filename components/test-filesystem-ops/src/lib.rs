use exports::componentized::services_test_components::ops::{Error, Guest};

mod fs;

struct FilesystemOps;

impl Guest for FilesystemOps {
    async fn list(path: String) -> Result<Vec<String>, Error> {
        fs::read_dir(&path).await
    }

    async fn read(path: String) -> Result<Vec<u8>, Error> {
        fs::read(&path).await
    }

    async fn write(path: String, data: Vec<u8>) -> Result<(), Error> {
        fs::write(&path, data).await
    }

    async fn move_(from_path: String, to_path: String) -> Result<(), Error> {
        fs::rename(&from_path, &to_path).await
    }

    async fn delete(path: String) -> Result<(), Error> {
        fs::remove_file(&path).await
    }
}

wit_bindgen::generate!({
    path: "../../tests/wit",
    world: "filesystem-ops",
    generate_all
});

export!(FilesystemOps);
