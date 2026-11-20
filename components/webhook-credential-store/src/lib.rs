use std::collections::HashMap;

use exports::componentized::services::credential_store::{Credential, Error, Guest, ServiceId};
use wasi::config::store as config;
use wasi::http::{
    client::send,
    types::{FieldName, Headers, Method, Request, Response, Scheme},
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

impl Guest for WebhookCredentialStore {
    async fn fetch(id: ServiceId) -> Result<Vec<Credential>, Error> {
        let response =
            Self::make_request("/services/credentials/fetch", vec![("service-id", &id)]).await?;
        match response.get_status_code() {
            200 => {
                // the body is consumed whole, nothing is left to report
                let (_, consumed) = wit_future::new(|| Ok(()));
                let (body, trailers) = Response::consume_body(response, consumed);
                let body = body.collect().await;
                trailers
                    .await
                    .map_err(|e| Error::from(format!("http error: {e:?}")))?;
                let rawcreds: HashMap<String, String> =
                    serde_json::from_slice(&body).map_err(|e| e.to_string())?;

                let creds = rawcreds
                    .into_iter()
                    .map(|(key, value)| Credential { key, value })
                    .collect();

                Ok(creds)
            }
            code => Err(Error::from(format!("unexpected http status {code}"))),
        }
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "webhook-credential-store",
    features: ["clocks-timezone"],
    generate_all
});

export!(WebhookCredentialStore);
