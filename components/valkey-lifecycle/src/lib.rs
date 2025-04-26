#![no_main]

use chrono::DateTime;
use componentized::services::credential_admin::{destroy, publish};
use componentized::services::types::{Credential, Request, Scope, Tier};
use componentized::valkey::store::{self as valkey};
use exports::componentized::services::lifecycle::{
    Error, Guest as Lifecycle, ServiceBindingId, ServiceInstanceId,
};
use wasi::clocks::wall_clock::now;
use wasi::config::store::{self as config};

const HOSTNAME_KEY: &str = "hostname";
const HOSTNAME_DEFAULT: &str = "127.0.0.1";
const PORT_KEY: &str = "port";
const PORT_DEFAULT: &str = "6379";
const USERNAME_KEY: &str = "username";
const USERNAME_DEFAULT: &str = "default";
const PASSWORD_KEY: &str = "password";

#[derive(Debug, Clone)]
struct ValkeyService {}

impl ValkeyService {
    fn instances_hash_key() -> String {
        "instances".to_string()
    }
    fn instance_data_key_prefix(instance_id: ServiceInstanceId) -> String {
        format!("instances:{instance_id}:")
    }
    fn instance_bindings_hash_key(instance_id: ServiceInstanceId) -> String {
        format!("instances:{instance_id}")
    }

    fn connect() -> Result<valkey::Connection, Error> {
        let host = Self::hostname()?;
        let port = Self::port()?;

        let opts = valkey::HelloOpts {
            proto_ver: Some("3".to_string()),
            auth: match config::get(PASSWORD_KEY)? {
                Some(password) => {
                    let username: String =
                        config::get(USERNAME_KEY)?.unwrap_or(USERNAME_DEFAULT.to_string());
                    Some((username, password))
                }
                None => None,
            },
            client_name: None,
        };
        let connection = valkey::connect(&host, port, Some(&opts))?;

        Ok(connection)
    }
    fn hostname() -> Result<String, Error> {
        let hostname = config::get(HOSTNAME_KEY)?.unwrap_or(String::from(HOSTNAME_DEFAULT));
        Ok(hostname)
    }
    fn port() -> Result<u16, Error> {
        let port = config::get(PORT_KEY)?.unwrap_or(String::from(PORT_DEFAULT));
        let port: u16 = port
            .parse()
            .map_err(|_| Error::from("port must be an integer"))?;
        Ok(port)
    }
}

impl Lifecycle for ValkeyService {
    fn provision(
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

        let connection = Self::connect()?;

        connection.hset(&Self::instances_hash_key(), &instance_id, "valkey")?;

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
        for binding_id in Self::list_bindings(instance_id.clone())? {
            Self::unbind(binding_id, instance_id.clone())?;
        }

        let connection = Self::connect()?;

        connection.hdel(&Self::instances_hash_key(), &instance_id)?;

        if !retain.unwrap_or(false) {
            let data_key_prefix = Self::instance_data_key_prefix(instance_id);
            for key in connection.keys(format!("{data_key_prefix}*").as_str())? {
                connection.del(&key)?;
            }
        }

        Ok(())
    }

    fn bind(
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

        let connection = Self::connect()?;

        let key_prefix = Self::instance_data_key_prefix(instance_id.clone());

        let hostname = Self::hostname()?;
        let port = Self::port()?;
        let username = binding_id.clone();
        let password = connection.acl_genpass()?;

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

        publish(&binding_id, credentials.as_slice())?;

        let mut rules = vec!["on".to_string(), format!(">{password}")];
        for scope in scopes {
            rules.push(format!("+@{scope}"));
        }
        rules.push(format!("~{key_prefix}*"));

        connection.acl_setuser(&username, &rules)?;
        connection.hset(
            &Self::instance_bindings_hash_key(instance_id.clone()),
            &binding_id,
            "valkey",
        )?;

        Ok(())
    }

    fn unbind(binding_id: ServiceBindingId, instance_id: ServiceInstanceId) -> Result<(), Error> {
        let connection = Self::connect()?;

        connection.acl_deluser(&binding_id)?;
        connection.hdel(&Self::instance_bindings_hash_key(instance_id), &binding_id)?;

        destroy(&binding_id)?;

        Ok(())
    }

    fn list_bindings(instance_id: ServiceInstanceId) -> Result<Vec<ServiceBindingId>, Error> {
        Ok(Self::connect()?.hkeys(&Self::instance_bindings_hash_key(instance_id))?)
    }
}

impl From<config::Error> for Error {
    fn from(e: config::Error) -> Self {
        match e {
            config::Error::Upstream(msg) => Self::from(format!("Config store Upstream: {msg}")),
            config::Error::Io(msg) => Self::from(format!("Config store IO: {msg}")),
        }
    }
}

impl From<valkey::Error> for Error {
    fn from(e: valkey::Error) -> Self {
        match e {
            valkey::Error::Client(msg) => Self::from(format!("Valkey store Client: {msg}")),
            valkey::Error::Resp(msg) => Self::from(format!("Valkey store RESP: {msg}")),
            valkey::Error::Valkey(msg) => Self::from(format!("Valkey store Valkey: {msg}")),
        }
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "valkey-lifecycle",
    generate_all
});

export!(ValkeyService);
