use componentized::secrets::store::{self as secrets, ErrorCode, Secret};
use exports::wasi::config::store::{Error, Guest};

/// The only key of the config, the root of the chroot.
const PATH_KEY: &str = "path";

pub(crate) struct FilesystemChrootConfig;

impl FilesystemChrootConfig {
    /// The `path` secret, none when the store has no `path`.
    ///
    /// The store must already be initialized, `init` is async and the config is sync.
    fn path() -> Result<Option<String>, Error> {
        match secrets::get(PATH_KEY) {
            Ok(Secret::String(path)) => Ok(Some(path)),
            Ok(Secret::Bytes(path)) => String::from_utf8(path)
                .map(Some)
                .map_err(|_| Error::Upstream(format!("secrets: '{PATH_KEY}' is not utf-8 text"))),
            Err(ErrorCode::NotFound) => Ok(None),
            Err(ErrorCode::NotInitialized) => Err(Error::Upstream(String::from(
                "secrets: the store is not initialized",
            ))),
            Err(ErrorCode::Other(Some(message))) => {
                Err(Error::Upstream(format!("secrets: {message}")))
            }
            Err(ErrorCode::Other(None)) => Err(Error::Upstream(format!(
                "secrets: unable to get '{PATH_KEY}'"
            ))),
        }
    }
}

impl Guest for FilesystemChrootConfig {
    fn get(key: String) -> Result<Option<String>, Error> {
        match key.as_str() {
            PATH_KEY => Self::path(),
            // nothing else is exposed, the store isn't consulted
            _ => Ok(None),
        }
    }

    fn get_all() -> Result<Vec<(String, String)>, Error> {
        Ok(Self::path()?
            .map(|path| (String::from(PATH_KEY), path))
            .into_iter()
            .collect())
    }
}

wit_bindgen::generate!({
    path: "../../tests/wit",
    world: "filesystem-chroot-config",
    generate_all
});

export!(FilesystemChrootConfig);
