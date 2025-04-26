#![no_main]

use chrono::DateTime;
use componentized::services::credential_admin::{destroy, publish};
use componentized::services::types::{Credential, Request, Scope, Tier};
use exports::componentized::services::lifecycle::{
    Error, Guest as Lifecycle, ServiceBindingId, ServiceInstanceId,
};
use std::fs;
use std::path::PathBuf;
use wasi::clocks::wall_clock::now;

const PATH_KEY: &str = "path";
const PATH_DEFAULT: &str = "services";
const INSTANCES_PATH_COMPONENT: &str = "instances";
const BINDINGS_PATH_COMPONENT: &str = "bindings";
const DATA_PATH_COMPONENT: &str = "data";

#[derive(Debug, Clone)]
struct FilesystemService {}

impl FilesystemService {
    fn get_instance_path(instance_id: ServiceInstanceId) -> Result<PathBuf, Error> {
        let base_path = wasi::config::store::get(PATH_KEY)
            .map_err(|e| Error::from(e.to_string()))?
            .unwrap_or(String::from(PATH_DEFAULT));

        Ok(PathBuf::new()
            .join(base_path)
            .join(INSTANCES_PATH_COMPONENT)
            .join(instance_id))
    }
    fn get_data_path(instance_id: ServiceInstanceId) -> Result<PathBuf, Error> {
        Ok(FilesystemService::get_instance_path(instance_id)?.join(DATA_PATH_COMPONENT))
    }
    fn get_bindings_path(instance_id: ServiceInstanceId) -> Result<PathBuf, Error> {
        Ok(FilesystemService::get_instance_path(instance_id)?.join(BINDINGS_PATH_COMPONENT))
    }
    fn get_binding_path(
        instance_id: ServiceInstanceId,
        binding_id: ServiceBindingId,
    ) -> Result<PathBuf, Error> {
        Ok(FilesystemService::get_bindings_path(instance_id)?.join(binding_id))
    }
}

impl Lifecycle for FilesystemService {
    fn provision(
        instance_id: ServiceInstanceId,
        type_: String,
        tier: Option<Tier>,
        requests: Option<Vec<Request>>,
    ) -> Result<(), Error> {
        if type_ != "filesystem" {
            Err(Error::from("only 'filesystem' types are supported"))?;
        }

        if tier.is_some() {
            Err(Error::from("tier is not supported"))?;
        }
        if requests.is_some() {
            Err(Error::from("requests are not supported"))?;
        }

        fs::create_dir_all(FilesystemService::get_data_path(instance_id.clone())?)
            .map_err(|e| Error::from(e.to_string()))?;
        fs::create_dir_all(FilesystemService::get_bindings_path(instance_id.clone())?)
            .map_err(|e| Error::from(e.to_string()))?;

        Ok(())
    }

    fn update(
        _instance_id: ServiceInstanceId,
        tier: Option<Tier>,
        requests: Option<Vec<Request>>,
    ) -> Result<(), Error> {
        if tier.is_some() {
            Err(Error::from("tier is not supported"))?;
        }
        if requests.is_some() {
            Err(Error::from("requests are not supported"))?;
        }

        // nothing is updatable
        Ok(())
    }

    fn destroy(instance_id: ServiceInstanceId, retain: Option<bool>) -> Result<(), Error> {
        fs::remove_dir(FilesystemService::get_bindings_path(instance_id.clone())?)
            .map_err(|e| Error::from(e.to_string()))?;

        if retain.unwrap_or(false) {
            return Ok(());
        }

        fs::remove_dir_all(FilesystemService::get_instance_path(instance_id.clone())?)
            .map_err(|e| Error::from(e.to_string()))?;

        Ok(())
    }

    fn bind(
        binding_id: ServiceBindingId,
        instance_id: ServiceInstanceId,
        scopes: Option<Vec<Scope>>,
    ) -> Result<(), Error> {
        if scopes.is_some() {
            Err(Error::from("scopes are not supported"))?;
        }

        let credentials = vec![
            Credential {
                key: String::from("type"),
                value: String::from("filesystem"),
            },
            Credential {
                key: String::from("path"),
                value: FilesystemService::get_binding_path(
                    instance_id.clone(),
                    binding_id.clone(),
                )?
                .into_os_string()
                .into_string()
                .unwrap(),
            },
            Credential {
                key: String::from("instance-id"),
                value: instance_id.clone(),
            },
            Credential {
                key: String::from("binding-id"),
                value: binding_id.clone(),
            },
            Credential {
                key: String::from("issued-at"),
                value: DateTime::from_timestamp(now().seconds as i64, 0)
                    .expect("valid wall clock time")
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            },
        ];

        // TODO soft_link is deprecated, but the replacements are not supported for wasm32
        #[allow(deprecated)]
        fs::soft_link(
            "../data",
            FilesystemService::get_binding_path(instance_id.clone(), binding_id.clone())?,
        )
        .map_err(|e| Error::from(e.to_string()))?;
        publish(&binding_id, credentials.as_slice())?;

        Ok(())
    }

    fn unbind(binding_id: ServiceBindingId, instance_id: ServiceInstanceId) -> Result<(), Error> {
        destroy(&binding_id)?;
        fs::remove_file(FilesystemService::get_binding_path(
            instance_id,
            binding_id,
        )?)
        .map_err(|e| Error::from(e.to_string()))
    }

    fn list_bindings(instance_id: ServiceInstanceId) -> Result<Vec<ServiceBindingId>, Error> {
        let dir = fs::read_dir(FilesystemService::get_bindings_path(instance_id.clone())?)
            .map_err(|e| Error::from(e.to_string()))?;
        let mut binding_ids: Vec<ServiceBindingId> = vec![];
        for file in dir {
            let file = file.map_err(|e| Error::from(e.to_string()))?;
            let binding_id: ServiceBindingId = file.file_name().to_str().unwrap().into();
            binding_ids.push(binding_id);
        }

        Ok(binding_ids)
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "filesystem-lifecycle",
    generate_all
});

export!(FilesystemService);
