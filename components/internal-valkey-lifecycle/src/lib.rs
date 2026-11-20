use crate::{
    componentized::{
        secrets::store as secrets,
        services::{
            credential_admin::{destroy, publish},
            types::{Credential, Request, Scope, Tier},
        },
        valkey::{
            commands,
            connector::{connect, Connection},
            tcp_connector_constants as constants,
            types::{
                AclCommandRule, AclKeyRule, AclRule, AclUserRule, ErrorCode as ValkeyErrorCode,
                StringOrBytes,
            },
        },
    },
    exports::componentized::services::lifecycle::{
        Error, Guest as Lifecycle, ServiceBindingId, ServiceInstanceId,
    },
    wasi::clocks::system_clock::now,
};

use chrono::DateTime;

#[derive(Debug, Clone)]
struct ValkeyService {}

impl ValkeyService {
    async fn provision_with_connection(
        connection: &Connection,
        instance_id: ServiceInstanceId,
        type_: String,
        tier: Option<Tier>,
        requests: Option<Vec<Request>>,
    ) -> Result<(), Error> {
        if type_ != "valkey" {
            Err(Error::from("only 'valkey' types are supported"))?;
        }

        if tier.is_some() {
            Err(Error::from("tier is not supported"))?;
        }
        if requests.is_some() {
            Err(Error::from("requests are not supported"))?;
        }

        commands::hset(
            &connection,
            instances_hash_key(),
            vec![(instance_id, StringOrBytes::from("valkey"))],
        )
        .await?;

        Ok(())
    }

    async fn destroy_with_connection(
        connection: &Connection,
        instance_id: ServiceInstanceId,
        retain: Option<bool>,
    ) -> Result<(), Error> {
        for binding_id in
            Self::list_bindings_with_connection(connection, instance_id.clone()).await?
        {
            Self::unbind_with_connection(connection, binding_id, instance_id.clone()).await?;
        }

        commands::hdel(&connection, instances_hash_key(), vec![instance_id.clone()]).await?;

        if !retain.unwrap_or(false) {
            let data_key_prefix = instance_data_key_prefix(instance_id);
            let keys = commands::keys(&connection, format!("{data_key_prefix}*")).await?;
            commands::del(&connection, keys).await?;
        }

        Ok(())
    }

    async fn bind_with_connection(
        connection: &Connection,
        binding_id: ServiceBindingId,
        instance_id: ServiceInstanceId,
        scopes: Option<Vec<Scope>>,
    ) -> Result<(), Error> {
        // default and validate scopes
        let scopes = scopes.unwrap_or(vec![Scope::from("read"), Scope::from("write")]);
        for scope in scopes.clone() {
            if scope != "read" && scope != "write" {
                Err(Error::from("a scope must be one of: read, write"))?;
            }
        }

        let key_prefix = instance_data_key_prefix(instance_id.clone());

        secrets::init().await?;

        let hostname =
            secret(&constants::hostname_key())?.unwrap_or_else(constants::hostname_default);
        let port = match secret(&constants::port_key())? {
            Some(port) => port.parse().map_err(|_| {
                Error::from(format!(
                    "{} must be a port number: {port}",
                    constants::port_key()
                ))
            })?,
            None => constants::port_default(),
        };
        let username = binding_id.clone();
        let password = commands::acl_genpass(&connection, None).await?;

        let credentials = vec![
            Credential {
                key: String::from("type"),
                value: String::from("valkey"),
            },
            Credential {
                key: String::from("hostname"),
                value: hostname,
            },
            Credential {
                key: String::from("port"),
                value: port.to_string(),
            },
            Credential {
                key: String::from("username"),
                value: username.clone(),
            },
            Credential {
                key: String::from("password"),
                value: password.clone(),
            },
            Credential {
                key: String::from("key-prefix"),
                value: key_prefix.clone(),
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
                key: String::from("scopes"),
                value: scopes.join(","),
            },
            Credential {
                key: String::from("issued-at"),
                value: DateTime::from_timestamp(now().seconds as i64, 0)
                    .expect("valid wall clock time")
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            },
        ];

        publish(binding_id.clone(), credentials).await?;

        let mut rules = vec![
            AclRule::User(AclUserRule::On),
            AclRule::User(AclUserRule::Password(password)),
        ];
        for scope in scopes {
            rules.push(AclRule::Command(AclCommandRule::AddCategory(scope)));
        }
        rules.push(AclRule::Keys(AclKeyRule::Add(format!("{key_prefix}*"))));

        commands::acl_setuser(&connection, username, Some(rules)).await?;
        commands::hset(
            &connection,
            instance_bindings_hash_key(instance_id.clone()),
            vec![(binding_id, StringOrBytes::from("valkey"))],
        )
        .await?;

        Ok(())
    }

    async fn unbind_with_connection(
        connection: &Connection,
        binding_id: ServiceBindingId,
        instance_id: ServiceInstanceId,
    ) -> Result<(), Error> {
        commands::acl_deluser(&connection, vec![binding_id.clone()]).await?;
        commands::hdel(
            &connection,
            instance_bindings_hash_key(instance_id),
            vec![binding_id.clone()],
        )
        .await?;

        destroy(binding_id).await?;

        Ok(())
    }

    async fn list_bindings_with_connection(
        connection: &Connection,
        instance_id: ServiceInstanceId,
    ) -> Result<Vec<ServiceBindingId>, Error> {
        Ok(commands::hkeys(&connection, instance_bindings_hash_key(instance_id)).await?)
    }
}

impl Lifecycle for ValkeyService {
    async fn provision(
        instance_id: ServiceInstanceId,
        type_: String,
        tier: Option<Tier>,
        requests: Option<Vec<Request>>,
    ) -> Result<(), Error> {
        let connection = connect().await?;
        Self::provision_with_connection(&connection, instance_id, type_, tier, requests).await
    }

    async fn update(
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

    async fn destroy(instance_id: ServiceInstanceId, retain: Option<bool>) -> Result<(), Error> {
        let connection = connect().await?;
        Self::destroy_with_connection(&connection, instance_id, retain).await
    }

    async fn bind(
        binding_id: ServiceBindingId,
        instance_id: ServiceInstanceId,
        scopes: Option<Vec<Scope>>,
    ) -> Result<(), Error> {
        let connection = connect().await?;
        Self::bind_with_connection(&connection, binding_id, instance_id, scopes).await
    }

    async fn unbind(
        binding_id: ServiceBindingId,
        instance_id: ServiceInstanceId,
    ) -> Result<(), Error> {
        let connection = connect().await?;
        Self::unbind_with_connection(&connection, binding_id, instance_id).await
    }

    async fn list_bindings(instance_id: ServiceInstanceId) -> Result<Vec<ServiceBindingId>, Error> {
        let connection = connect().await?;
        Self::list_bindings_with_connection(&connection, instance_id).await
    }
}

fn instances_hash_key() -> String {
    "instances".to_string()
}
fn instance_data_key_prefix(instance_id: ServiceInstanceId) -> String {
    format!("instances:{instance_id}:")
}
fn instance_bindings_hash_key(instance_id: ServiceInstanceId) -> String {
    format!("instances:{instance_id}")
}

/// The value of the lifecycle's secret, none when it is not found.
///
/// The store must be initialized before this method is called.
fn secret(key: &str) -> Result<Option<String>, Error> {
    match secrets::get(key) {
        Ok(secrets::Secret::String(secret)) => Ok(Some(secret)),
        Ok(secrets::Secret::Bytes(_)) => Err(Error::from(format!(
            "secrets: value for {key} must be a string "
        ))),
        Err(secrets::ErrorCode::NotFound) => Ok(None),
        Err(secrets::ErrorCode::NotInitialized) => Err(Error::from("secrets: not initialized")),
        Err(secrets::ErrorCode::Other(None)) => Err(Error::from("secrets: other".to_string())),
        Err(secrets::ErrorCode::Other(Some(message))) => Err(Error::from(message)),
    }
}

impl From<ValkeyErrorCode> for Error {
    fn from(e: ValkeyErrorCode) -> Self {
        match e {
            componentized::valkey::types::ErrorCode::Closed(None) => Self::from("closed"),
            componentized::valkey::types::ErrorCode::Closed(Some(message)) => {
                Self::from(format!("closed: {message}"))
            }
            componentized::valkey::types::ErrorCode::Client(message) => {
                Self::from(format!("client: {message}"))
            }
            componentized::valkey::types::ErrorCode::Resp(message) => {
                Self::from(format!("resp: {message}"))
            }
            componentized::valkey::types::ErrorCode::Server(StringOrBytes::String(message)) => {
                format!("server: {message}")
            }
            componentized::valkey::types::ErrorCode::Server(StringOrBytes::Bytes(_)) => {
                format!("server")
            }
            componentized::valkey::types::ErrorCode::Lagged => Self::from("lagged"),
        }
    }
}

impl From<secrets::ErrorCode> for Error {
    fn from(e: secrets::ErrorCode) -> Self {
        match e {
            componentized::secrets::types::ErrorCode::NotInitialized => {
                Self::from("secrets: not-initialized")
            }
            componentized::secrets::types::ErrorCode::NotFound => Self::from("secrets: not-found"),
            componentized::secrets::types::ErrorCode::Other(None) => Self::from("secrets"),
            componentized::secrets::types::ErrorCode::Other(Some(message)) => {
                Self::from(format!("secrets: {message}"))
            }
        }
    }
}

impl From<&str> for StringOrBytes {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "valkey-lifecycle",
    features: ["clocks-timezone"],
    generate_all
});

export!(ValkeyService);
