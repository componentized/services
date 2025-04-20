#![no_main]

use componentized::services::lifecycle;
use componentized::services::types::{Request, ServiceInstanceId};
use exports::wasi::http::incoming_handler::Guest;
use wasi::http::types::{
    ErrorCode, Fields, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use wasi::logging::logging::{log, Level};

struct LifecycleHost;

impl Guest for LifecycleHost {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let headers = Fields::new();
        let response = OutgoingResponse::new(headers);
        let body = response.body().unwrap();

        let path_with_query = request.path_with_query().unwrap();
        let parts: Vec<&str> = path_with_query.splitn(2, "?").collect();
        let path = *parts.get(0).unwrap();
        let query = querystring::querify(*parts.get(1).unwrap_or(&""));

        match path {
            "/provision" => {
                let ctx: Vec<u8> = get_param(&query, "context")
                    .map(|c| c.into_bytes())
                    .unwrap_or(vec![]);
                let type_ = get_param(&query, "type").unwrap_or("".to_string());
                let tier = get_param(&query, "tier");
                let requests: Option<Vec<Request>> =
                    get_params(&query, "requests").map(|requests| {
                        requests
                            .into_iter()
                            .map(|p| {
                                let parts: Vec<&str> = p.splitn(2, "=").collect();
                                let key = *parts.get(0).unwrap();
                                let value = *parts.get(1).unwrap_or(&"");
                                Request {
                                    key: key.to_string(),
                                    value: value.to_string(),
                                }
                            })
                            .collect()
                    });
                log(Level::Info, "host", &format!("Provision {type_}"));
                match lifecycle::provision(&ctx, &type_, tier.as_ref(), requests.as_deref()) {
                    Ok(instance_id) => {
                        ResponseOutparam::set(response_out, Ok(response));
                        let out = body.write().expect("outgoing stream");
                        out.blocking_write_and_flush(format!("{}\n", instance_id).as_bytes())
                            .expect("writing response");
                    }
                    Err(e) => {
                        ResponseOutparam::set(response_out, Err(ErrorCode::InternalError(Some(e))));
                    }
                }
            }
            "/update" => {
                let instance_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "instance-id").unwrap_or("".to_string()),
                );
                let tier = get_param(&query, "tier");
                let requests: Option<Vec<Request>> =
                    get_params(&query, "requests").map(|requests| {
                        requests
                            .into_iter()
                            .map(|p| {
                                let parts: Vec<&str> = p.splitn(2, "=").collect();
                                let key = *parts.get(0).unwrap();
                                let value = *parts.get(1).unwrap_or(&"");
                                Request {
                                    key: key.to_string(),
                                    value: value.to_string(),
                                }
                            })
                            .collect()
                    });

                log(Level::Info, "host", &format!("Update {instance_id}"));
                match lifecycle::update(&instance_id, tier.as_ref(), requests.as_deref()) {
                    Ok(_) => {
                        ResponseOutparam::set(response_out, Ok(response));
                    }
                    Err(e) => {
                        ResponseOutparam::set(response_out, Err(ErrorCode::InternalError(Some(e))));
                    }
                }
            }
            "/destroy" => {
                let instance_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "instance-id").unwrap_or("".to_string()),
                );
                let retain = get_param(&query, "retain").map(|r| r.parse().unwrap_or(false));
                log(
                    Level::Info,
                    "host",
                    &format!("Destroy {instance_id} {retain:?}"),
                );
                match lifecycle::destroy(&instance_id, retain) {
                    Ok(_) => {
                        ResponseOutparam::set(response_out, Ok(response));
                    }
                    Err(e) => {
                        ResponseOutparam::set(response_out, Err(ErrorCode::InternalError(Some(e))));
                    }
                }
            }
            "/bind" => {
                let ctx: Vec<u8> = get_param(&query, "context")
                    .map(|c| c.into_bytes())
                    .unwrap_or(vec![]);
                let instance_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "instance-id").unwrap_or("".to_string()),
                );
                let scopes = get_params(&query, "scopes");
                log(
                    Level::Info,
                    "host",
                    &format!("Bind {instance_id}: {scopes:?}"),
                );
                match lifecycle::bind(&ctx, &instance_id, scopes.as_deref()) {
                    Ok(binding_id) => {
                        ResponseOutparam::set(response_out, Ok(response));
                        let out = body.write().expect("outgoing stream");
                        out.blocking_write_and_flush(format!("{}\n", binding_id).as_bytes())
                            .expect("writing response");
                    }
                    Err(e) => {
                        ResponseOutparam::set(response_out, Err(ErrorCode::InternalError(Some(e))));
                    }
                }
            }
            "/list-bindings" => {
                let instance_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "instance-id").unwrap_or("".to_string()),
                );
                log(Level::Info, "host", &format!("List bindings {instance_id}"));
                match lifecycle::list_bindings(&instance_id) {
                    Ok(binding_ids) => {
                        ResponseOutparam::set(response_out, Ok(response));
                        let out = body.write().expect("outgoing stream");
                        out.blocking_write_and_flush(
                            format!("{}\n", binding_ids.join("\n")).as_bytes(),
                        )
                        .expect("writing response");
                    }
                    Err(e) => {
                        ResponseOutparam::set(response_out, Err(ErrorCode::InternalError(Some(e))));
                    }
                }
            }
            "/unbind" => {
                let binding_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "binding-id").unwrap_or("".to_string()),
                );
                log(Level::Info, "host", &format!("Unbind {binding_id}"));
                match lifecycle::unbind(&binding_id) {
                    Ok(_) => {
                        ResponseOutparam::set(response_out, Ok(response));
                    }
                    Err(e) => {
                        ResponseOutparam::set(response_out, Err(ErrorCode::InternalError(Some(e))));
                    }
                }
            }
            path => {
                log(Level::Warn, "http", &format!("unmapped path: {path}"));
                ResponseOutparam::set(
                    response_out,
                    Err(ErrorCode::InternalError(Some("Not Found".to_string()))),
                );
            }
        };

        OutgoingBody::finish(body, None).unwrap();
    }
}

fn get_param(query: &querystring::QueryParams, key: &str) -> Option<String> {
    for (k, v) in query {
        if *k == key {
            return Some(v.to_string());
        }
    }
    None
}

fn get_params(query: &querystring::QueryParams, key: &str) -> Option<Vec<String>> {
    let mut values = vec![];
    for (k, v) in query {
        if *k == key {
            values.push(v.to_string());
        }
    }
    match values.len() {
        0 => None,
        _ => Some(values),
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "lifecycle-host-http",
    generate_all
});

export!(LifecycleHost);
