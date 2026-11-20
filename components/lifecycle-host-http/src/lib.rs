use componentized::services::lifecycle;
use componentized::services::types::{Request, ServiceBindingId, ServiceInstanceId};
use exports::wasi::http::handler::Guest;
use wasi::http::types::{self as http, ErrorCode, Fields, Response};
use wasi::logging::logging::{log, Level};

struct LifecycleHost;

impl Guest for LifecycleHost {
    async fn handle(request: http::Request) -> Result<Response, ErrorCode> {
        let path_with_query = request.get_path_with_query().unwrap_or_default();
        let parts: Vec<&str> = path_with_query.splitn(2, "?").collect();
        let path = *parts.get(0).unwrap();
        let query = querystring::querify(*parts.get(1).unwrap_or(&""));

        let body = match path {
            "/provision" => {
                let instance_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "instance-id").unwrap_or("".to_string()),
                );
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
                match lifecycle::provision(instance_id.clone(), type_, tier, requests).await {
                    Ok(_) => format!("{}\n", instance_id),
                    Err(e) => return Err(ErrorCode::InternalError(Some(e))),
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
                match lifecycle::update(instance_id, tier, requests).await {
                    Ok(_) => String::new(),
                    Err(e) => return Err(ErrorCode::InternalError(Some(e))),
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
                match lifecycle::destroy(instance_id, retain).await {
                    Ok(_) => String::new(),
                    Err(e) => return Err(ErrorCode::InternalError(Some(e))),
                }
            }
            "/bind" => {
                let binding_id: ServiceBindingId = ServiceBindingId::from(
                    get_param(&query, "binding-id").unwrap_or("".to_string()),
                );
                let instance_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "instance-id").unwrap_or("".to_string()),
                );
                let scopes = get_params(&query, "scopes");
                log(
                    Level::Info,
                    "host",
                    &format!("Bind {instance_id}: {scopes:?}"),
                );
                match lifecycle::bind(binding_id.clone(), instance_id, scopes).await {
                    Ok(_) => format!("{}\n", binding_id),
                    Err(e) => return Err(ErrorCode::InternalError(Some(e))),
                }
            }
            "/list-bindings" => {
                let instance_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "instance-id").unwrap_or("".to_string()),
                );
                log(Level::Info, "host", &format!("List bindings {instance_id}"));
                match lifecycle::list_bindings(instance_id).await {
                    Ok(binding_ids) => format!("{}\n", binding_ids.join("\n")),
                    Err(e) => return Err(ErrorCode::InternalError(Some(e))),
                }
            }
            "/unbind" => {
                let binding_id: ServiceBindingId = ServiceBindingId::from(
                    get_param(&query, "binding-id").unwrap_or("".to_string()),
                );
                let instance_id: ServiceInstanceId = ServiceInstanceId::from(
                    get_param(&query, "instance-id").unwrap_or("".to_string()),
                );
                log(Level::Info, "host", &format!("Unbind {binding_id}"));
                match lifecycle::unbind(binding_id, instance_id).await {
                    Ok(_) => String::new(),
                    Err(e) => return Err(ErrorCode::InternalError(Some(e))),
                }
            }
            path => {
                log(Level::Warn, "http", &format!("unmapped path: {path}"));
                return Err(ErrorCode::InternalError(Some("Not Found".to_string())));
            }
        };

        Ok(respond(body))
    }
}

/// A response with the body, written once the response is returned.
fn respond(body: String) -> Response {
    // no trailers once the writer is dropped
    let (_, trailers) = wit_future::new(|| Ok(None));
    if body.is_empty() {
        let (response, _) = Response::new(Fields::new(), None, trailers);
        return response;
    }
    let (mut writer, contents) = wit_stream::new();
    let (response, _) = Response::new(Fields::new(), Some(contents), trailers);
    wit_bindgen::spawn_local(async move {
        writer.write_all(body.into_bytes()).await;
    });
    response
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
    path: "../wit",
    world: "lifecycle-host-http",
    features: ["clocks-timezone"],
    generate_all
});

export!(LifecycleHost);
