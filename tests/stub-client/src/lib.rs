#![no_main]

use exports::greeter::Guest;
use wasi::logging::logging::{log, Level};

#[derive(Debug, Clone)]
struct StubClient {}

impl Guest for StubClient {
    fn greet(name: String) -> Result<String, String> {
        log(Level::Info, "stub-client", "greet: {name}");
        let creds = wasi::config::store::get_all().map_err(|e| e.to_string())?;
        for (key, value) in creds {
            log(
                Level::Info,
                "stub-client",
                &format!("credential {key}: {value}"),
            );
        }
        Ok(format!("Hello {name}!"))
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "stub-client",
    generate_all
});

export!(StubClient);
