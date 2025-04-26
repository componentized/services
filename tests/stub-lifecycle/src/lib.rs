#![no_main]

use std::vec;

use componentized::services::credential_admin;
use componentized::services::types::{Request, Scope, Tier};
use exports::componentized::services::lifecycle::{
    Error, Guest as Lifecycle, ServiceBindingId, ServiceInstanceId,
};
use wasi::logging::logging::{log, Level};

#[derive(Debug, Clone)]
struct StubService {}

impl Lifecycle for StubService {
    fn provision(
        instance_id: ServiceInstanceId,
        type_: String,
        tier: Option<Tier>,
        requests: Option<Vec<Request>>,
    ) -> Result<(), Error> {
        log(Level::Info, "lifecycle", &format!("provision: instance-id={instance_id} type={type_} tier={tier:?} requests={requests:?}"));

        Ok(())
    }

    fn update(
        instance_id: ServiceInstanceId,
        tier: Option<Tier>,
        requests: Option<Vec<Request>>,
    ) -> Result<(), Error> {
        log(
            Level::Info,
            "lifecycle",
            &format!("update: instance-id={instance_id} tier={tier:?} requests={requests:?}"),
        );
        Ok(())
    }

    fn destroy(instance_id: ServiceInstanceId, retain: Option<bool>) -> Result<(), Error> {
        log(
            Level::Info,
            "lifecycle",
            &format!("destroy: instance-id={instance_id} retain={retain:?}"),
        );

        Ok(())
    }

    fn bind(
        binding_id: ServiceBindingId,
        instance_id: ServiceInstanceId,
        scopes: Option<Vec<Scope>>,
    ) -> Result<(), Error> {
        credential_admin::publish(&binding_id, &vec![])?;
        log(
            Level::Info,
            "lifecycle",
            &format!("bind: instance-id={instance_id} binding-id={binding_id} scopes={scopes:?}"),
        );
        Ok(())
    }

    fn unbind(binding_id: ServiceBindingId, instance_id: ServiceInstanceId) -> Result<(), Error> {
        credential_admin::destroy(&binding_id)?;
        log(
            Level::Info,
            "lifecycle",
            &format!("unbind: binding-id={binding_id} instance-id={instance_id}"),
        );
        Ok(())
    }

    fn list_bindings(instance_id: ServiceInstanceId) -> Result<Vec<ServiceBindingId>, Error> {
        log(
            Level::Info,
            "lifecycle",
            &format!("list-bindings: instance-id={instance_id}"),
        );
        Ok(vec![])
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "stub-lifecycle",
    generate_all
});

export!(StubService);
