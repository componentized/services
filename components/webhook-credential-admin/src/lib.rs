#![no_main]

use std::collections::HashMap;

use exports::componentized::services::credential_admin::{Credential, Error, Guest, ServiceId};
use wasi::config::store as config;
use wasi::http::{
    outgoing_handler::handle,
    types::{FieldName, Headers, IncomingResponse, Method, OutgoingRequest, Scheme},
};

const HOST_KEY: &str = "host";
const SCHEME_KEY: &str = "scheme";

pub(crate) struct WebhookCredentialAdmin {}

impl WebhookCredentialAdmin {
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
            Ok(None) => Err(Error::from("'host' config value is required")),
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

impl Guest for WebhookCredentialAdmin {
    fn publish(id: ServiceId, credentials: Vec<Credential>) -> Result<(), Error> {
        let mut creds = HashMap::new();
        for cred in credentials {
            creds.insert(cred.key, cred.value);
        }
        let service_credentials = serde_json::to_string(&creds).unwrap();

        let response = Self::make_request(
            "/services/credentials/publish",
            vec![
                ("service-id", &id),
                ("service-credentials", &service_credentials),
            ],
        )?;
        match response.status() {
            204 => Ok(()),
            code => Err(Error::from(format!("unexpected http status {code}"))),
        }
    }

    fn destroy(id: ServiceId) -> Result<(), Error> {
        let response =
            Self::make_request("/services/credentials/destroy", vec![("service-id", &id)])?;
        match response.status() {
            204 => Ok(()),
            code => Err(Error::from(format!("unexpected http status {code}"))),
        }
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "webhook-credential-admin",
    features: ["clocks-timezone"],
    generate_all
});

export!(WebhookCredentialAdmin);
