use std::collections::HashMap;
use std::sync::OnceLock;

use componentized::services::credential_store::fetch;
use exports::componentized::secrets::store::{ErrorCode, Guest, Secret};
use wasi::config::store::{self as config};

const BINDING_ID_KEY: &str = "binding-id";

/// The credential values, or the error reading them, read once by the first `init`.
static SECRETS: OnceLock<Result<HashMap<String, String>, ErrorCode>> = OnceLock::new();

pub(crate) struct CredentialConfig {}

impl Guest for CredentialConfig {
    async fn init() -> Result<(), ErrorCode> {
        if SECRETS.get().is_none() {
            // never resolve the first init immediately, so callers handle init as async work
            wit_bindgen::yield_async().await;
        }
        SECRETS
            .get_or_init(
                || match config::get(BINDING_ID_KEY).map_err(config_err_map)? {
                    None => Err(ErrorCode::Other(Some(format!(
                        "config: must contain '{}'",
                        BINDING_ID_KEY
                    )))),
                    Some(binding_id) => wit_bindgen::block_on(async move {
                        let credentials = fetch(binding_id)
                            .await
                            .map_err(|message| {
                                ErrorCode::Other(Some(format!("credential-store: {message}")))
                            })?
                            .iter()
                            .map(|c| (c.key.clone(), c.value.clone()))
                            .collect();

                        Ok(credentials)
                    }),
                },
            )
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone)
    }

    fn get(key: String) -> Result<Secret, ErrorCode> {
        let Some(Ok(secrets)) = SECRETS.get() else {
            return Err(ErrorCode::NotInitialized);
        };
        secrets
            .get(&key)
            .map(|value| Secret::String(value.clone()))
            .ok_or(ErrorCode::NotFound)
    }
}

fn config_err_map(err: config::Error) -> ErrorCode {
    match err {
        config::Error::Upstream(message) => {
            ErrorCode::Other(Some(format!("config: upstream {message}")))
        }
        config::Error::Io(message) => ErrorCode::Other(Some(format!("config: io {message}"))),
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "credential-config",
    features: ["clocks-timezone"],
    generate_all
});

export!(CredentialConfig);
