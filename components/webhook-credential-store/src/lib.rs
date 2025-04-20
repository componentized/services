#![no_main]

use std::collections::HashMap;

use exports::componentized::services::credential_store::{
    Credential, Error, Guest, ServiceBindingId,
};
use wasi::config::store as config;
use wasi::http::{
    outgoing_handler::handle,
    types::{FieldName, Headers, IncomingResponse, Method, OutgoingRequest, Scheme},
};

const HOST_KEY: &str = "host";
const SCHEME_KEY: &str = "scheme";

pub(crate) struct WebhookCredentialStore {}

impl WebhookCredentialStore {
    fn scheme() -> Result<Scheme, Error> {
        match config::get(SCHEME_KEY) {
            Ok(Some(scheme)) => match scheme.as_str() {
                "http" => Ok(Scheme::Http),
                "https" => Ok(Scheme::Https),
                other => Ok(Scheme::Other(other.to_string())),
            },
            Ok(None) => Ok(Scheme::Https),
            Err(e) => Err(Error::from(e.to_string()))?,
        }
    }
    fn host() -> Result<String, Error> {
        match config::get(HOST_KEY) {
            Ok(Some(host)) => Ok(host),
            Ok(None) => Err(Error::from(format!(
                "'{HOST_KEY}' config value is required"
            ))),
            Err(e) => Err(Error::from(e.to_string())),
        }
    }

    fn make_request(path: &str, headers: Vec<(&str, &str)>) -> Result<IncomingResponse, Error> {
        let request_headers = Headers::new();
        for (name, value) in headers {
            _ = request_headers
                .set(
                    &FieldName::from(name),
                    vec![value.as_bytes().to_vec()].as_ref(),
                )
                .map_err(|e| Error::from(e.to_string()))?;
        }
        let request = OutgoingRequest::new(request_headers);
        _ = request
            .set_method(&Method::Post)
            .map_err(|_| Error::from("unable to set method"))?;
        // TODO get scheme and authority from config
        _ = request
            .set_scheme(Some(&Self::scheme()?))
            .map_err(|_| Error::from("unable to set scheme"))?;
        _ = request
            .set_authority(Some(&Self::host()?))
            .map_err(|_| Error::from("unable to set authority"))?;
        _ = request
            .set_path_with_query(Some(&path))
            .map_err(|_| Error::from("unable to set path"))?;

        let response = handle(request, None).map_err(|e| Error::from(e.to_string()))?;
        response.subscribe().block();

        let response = response
            .get()
            .ok_or_else(|| Error::from("unable to get response"))?
            .map_err(|_| Error::from("http error"))?
            .map_err(|e| Error::from(e.to_string()))?;
        Ok(response)
    }
}

impl Guest for WebhookCredentialStore {
    fn fetch(binding_id: ServiceBindingId) -> Result<Vec<Credential>, Error> {
        let response = Self::make_request(
            "/services/credentials/fetch",
            vec![("service-binding-id", &binding_id)],
        )?;
        match response.status() {
            200 => {
                let body = response.consume().unwrap();
                let body_stream = body.stream().unwrap();
                body_stream.subscribe().block();
                let rawcreds: HashMap<String, String> = serde_json::from_slice(
                    body_stream.blocking_read(1024 * 1024).unwrap().as_ref(),
                )
                .map_err(|e| e.to_string())?;

                let mut creds = vec![];
                for (key, value) in rawcreds {
                    creds.push(Credential { key, value });
                }

                drop(body_stream);
                drop(body);

                Ok(creds)
            }
            code => Err(Error::from(format!("unexpected http status {code}"))),
        }
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "webhook-credential-store",
    generate_all
});

export!(WebhookCredentialStore);
