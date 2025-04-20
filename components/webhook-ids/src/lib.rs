#![no_main]

use exports::componentized::services::ids::{
    Context, Error, Guest, ServiceBindingId, ServiceInstanceId,
};
use wasi::config::store as config;
use wasi::http::{
    outgoing_handler::handle,
    types::{FieldName, Headers, IncomingResponse, Method, OutgoingRequest, Scheme},
};

const HOST_KEY: &str = "host";
const SCHEME_KEY: &str = "scheme";

pub(crate) struct WebhookIds {}

impl WebhookIds {
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

impl Guest for WebhookIds {
    fn generate_instance_id(ctx: Context) -> Result<ServiceInstanceId, Error> {
        let response = Self::make_request(
            "/services/ids/instance",
            vec![("context", &String::from_utf8_lossy(&ctx))],
        )?;
        let headers = response.headers();
        let instance_ids = headers.get(&FieldName::from("service-instance-id"));
        let instance_id = instance_ids
            .first()
            .ok_or("missing expected response header: service-instance-id")?
            .as_ref();
        Ok(String::from_utf8_lossy(instance_id).to_string())
    }

    fn generate_binding_id(
        ctx: Context,
        instance_id: ServiceInstanceId,
    ) -> Result<ServiceBindingId, Error> {
        let response = Self::make_request(
            "/services/ids/binding",
            vec![
                ("context", &String::from_utf8_lossy(&ctx)),
                ("service-instance-id", &instance_id),
            ],
        )?;
        let headers = response.headers();
        let instance_ids = headers.get(&FieldName::from("service-binding-id"));
        let instance_id = instance_ids
            .first()
            .ok_or("missing expected response header: service-binding-id")?
            .as_ref();
        Ok(String::from_utf8_lossy(instance_id).to_string())
    }

    fn lookup_instance_id(binding_id: ServiceBindingId) -> Result<ServiceInstanceId, Error> {
        let response = Self::make_request(
            "/services/ids/lookup",
            vec![("service-binding-id", &binding_id)],
        )?;
        let headers = response.headers();
        let instance_ids = headers.get(&FieldName::from("service-instance-id"));
        let instance_id = instance_ids
            .first()
            .ok_or("missing expected response header: service-instance-id")?
            .as_ref();
        Ok(String::from_utf8_lossy(instance_id).to_string())
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "webhook-ids",
    generate_all
});

export!(WebhookIds);
