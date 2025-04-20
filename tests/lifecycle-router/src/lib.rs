#![no_main]

use componentized::services::types::Credential;
use exports::componentized::services::lifecycle::{
    Context, Error, Guest, Request, Scope, ServiceBindingId, ServiceInstanceId, Tier,
};

struct Lifecycle;

impl Lifecycle {
    fn get_lifecycle(type_: String) -> Result<LifeycleType, Error> {
        match type_.as_str() {
            "filesystem" => Ok(LifeycleType::Filesystem),
            "valkey" => Ok(LifeycleType::Keyvalue),
            _ => Err(Error::from(format!("Unknown type '{type_}'"))),
        }
    }
    fn get_type_for_instance_id(instance_id: ServiceInstanceId) -> Result<String, Error> {
        let creds = componentized::services::credential_store::fetch(&instance_id)?;
        let type_cred = creds.iter().find(|c| c.key == "type");
        match type_cred {
            Some(type_cred) => Ok(type_cred.value.clone()),
            None => Err(Error::from("Instance credentials must contain type")),
        }
    }
    fn get_type_for_binding_id(binding_id: ServiceBindingId) -> Result<String, Error> {
        let creds = componentized::services::credential_store::fetch(&binding_id)?;
        let type_cred = creds.iter().find(|c| c.key == "type");
        match type_cred {
            Some(type_cred) => Ok(type_cred.value.clone()),
            None => Err(Error::from("Binding credentials must contain type")),
        }
    }
}

enum LifeycleType {
    Filesystem,
    Keyvalue,
}

impl Guest for Lifecycle {
    fn provision(
        ctx: Context,
        type_: String,
        tier: Option<Tier>,
        requests: Option<Vec<Request>>,
    ) -> Result<ServiceInstanceId, Error> {
        let tier = tier.as_ref();
        let requests = requests.as_deref();
        let instance_id = match Lifecycle::get_lifecycle(type_.clone())? {
            LifeycleType::Filesystem => {
                filesystem_lifecycle::provision(&ctx, &type_, tier, requests)
            }
            LifeycleType::Keyvalue => keyvalue_lifecycle::provision(&ctx, &type_, tier, requests),
        }?;
        // mild abuse of the credential store by stashing the type for an instance rather than a binding
        componentized::services::credential_admin::publish(
            &instance_id,
            vec![Credential {
                key: "type".to_string(),
                value: type_,
            }]
            .as_ref(),
        )?;
        Ok(instance_id)
    }

    fn update(
        instance_id: ServiceInstanceId,
        tier: Option<Tier>,
        requests: Option<Vec<Request>>,
    ) -> Result<(), Error> {
        let type_ = Lifecycle::get_type_for_instance_id(instance_id.clone())?;
        let tier = tier.as_ref();
        let requests = requests.as_deref();
        match Lifecycle::get_lifecycle(type_.clone())? {
            LifeycleType::Filesystem => filesystem_lifecycle::update(&instance_id, tier, requests),
            LifeycleType::Keyvalue => keyvalue_lifecycle::update(&instance_id, tier, requests),
        }
    }

    fn destroy(instance_id: ServiceInstanceId, retain: Option<bool>) -> Result<(), Error> {
        let type_ = Lifecycle::get_type_for_instance_id(instance_id.clone())?;
        // mild abuse of the credential store by stashing the type for an instance rather than a binding
        componentized::services::credential_admin::destroy(&instance_id)?;
        match Lifecycle::get_lifecycle(type_.clone())? {
            LifeycleType::Filesystem => filesystem_lifecycle::destroy(&instance_id, retain),
            LifeycleType::Keyvalue => keyvalue_lifecycle::destroy(&instance_id, retain),
        }
    }

    fn bind(
        ctx: Context,
        instance_id: ServiceInstanceId,
        scopes: Option<Vec<Scope>>,
    ) -> Result<ServiceBindingId, Error> {
        let type_ = Lifecycle::get_type_for_instance_id(instance_id.clone())?;
        let scopes = scopes.as_deref();
        match Lifecycle::get_lifecycle(type_.clone())? {
            LifeycleType::Filesystem => filesystem_lifecycle::bind(&ctx, &instance_id, scopes),
            LifeycleType::Keyvalue => keyvalue_lifecycle::bind(&ctx, &instance_id, scopes),
        }
    }

    fn unbind(binding_id: ServiceBindingId) -> Result<(), Error> {
        let type_ = Lifecycle::get_type_for_binding_id(binding_id.clone())?;
        match Lifecycle::get_lifecycle(type_.clone())? {
            LifeycleType::Filesystem => filesystem_lifecycle::unbind(&binding_id),
            LifeycleType::Keyvalue => keyvalue_lifecycle::unbind(&binding_id),
        }
    }

    fn list_bindings(instance_id: ServiceInstanceId) -> Result<Vec<ServiceBindingId>, Error> {
        let type_ = Lifecycle::get_type_for_instance_id(instance_id.clone())?;
        match Lifecycle::get_lifecycle(type_.clone())? {
            LifeycleType::Filesystem => filesystem_lifecycle::list_bindings(&instance_id),
            LifeycleType::Keyvalue => keyvalue_lifecycle::list_bindings(&instance_id),
        }
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "lifecycle-router",
    generate_all
});

export!(Lifecycle);
