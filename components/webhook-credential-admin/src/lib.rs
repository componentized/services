use std::collections::HashMap;

use exports::componentized::services::credential_admin::{Credential, Error, Guest, ServiceId};
use wasi::config::store as config;
use wasi::http::{
    client::send,
    types::{FieldName, Headers, Method, Request, Response, Scheme},
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

    async fn make_request(path: &str, headers: Vec<(&str, &str)>) -> Result<Response, Error> {
        let request_headers = Headers::new();
        for (name, value) in headers {
            request_headers
                .set(&FieldName::from(name), &[value.as_bytes().to_vec()])
                .map_err(|e| Error::from(format!("invalid header '{name}': {e:?}")))?;
        }
        // no body, and no trailers once the writer is dropped
        let (_, trailers) = wit_future::new(|| Ok(None));
        let (request, _) = Request::new(request_headers, None, trailers, None);
        request
            .set_method(&Method::Post)
            .map_err(|_| Error::from("unable to set method"))?;
        request
            .set_scheme(Some(&Self::scheme()?))
            .map_err(|_| Error::from("unable to set scheme"))?;
        request
            .set_authority(Some(&Self::host()?))
            .map_err(|_| Error::from("unable to set authority"))?;
        request
            .set_path_with_query(Some(path))
            .map_err(|_| Error::from("unable to set path"))?;

        send(request)
            .await
            .map_err(|e| Error::from(format!("http error: {e:?}")))
    }
}

impl Guest for WebhookCredentialAdmin {
    async fn publish(id: ServiceId, credentials: Vec<Credential>) -> Result<(), Error> {
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
        )
        .await?;
        match response.get_status_code() {
            204 => Ok(()),
            code => Err(Error::from(format!("unexpected http status {code}"))),
        }
    }

    async fn destroy(id: ServiceId) -> Result<(), Error> {
        let response =
            Self::make_request("/services/credentials/destroy", vec![("service-id", &id)]).await?;
        match response.get_status_code() {
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
