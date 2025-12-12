use clap::{command, Parser, Subcommand};
use componentized::services::lifecycle;
use componentized::services::types::{
    Credential, Error, Request, Scope, ServiceBindingId, ServiceInstanceId, Tier,
};
use exports::wasi::cli::run::Guest;
use regex_lite::Regex;
use wasi::cli::environment;
use wasi::logging::logging::{log, Level};
use wasi::random::random::get_random_bytes;

/// componentized services CLI
#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "services")]
#[command(about = "componentized services CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, Clone)]
enum Commands {
    /// Provision a new service instance
    Provision {
        /// Identifier for the service instance
        #[arg(long)]
        instance_id: Option<ServiceInstanceId>,

        /// Type of service
        #[arg(short, long("type"))]
        type_: String,

        /// Tier for the service
        #[arg(long)]
        tier: Option<Tier>,

        /// Requested parameters for the service in key=value format
        #[arg(short, long)]
        requests: Option<Vec<Request>>,
    },

    /// Update a provisioned service
    #[command(arg_required_else_help = true)]
    Update {
        /// Identifier for the service instance
        #[arg(required = true)]
        instance_id: ServiceInstanceId,

        /// Tier for the service
        #[arg(short, long, default_value = "")]
        tier: Option<Tier>,

        /// Requested parameters for the service in key=value format
        #[arg(short, long)]
        requests: Option<Vec<Request>>,
    },

    /// Destroy a provisioned service
    #[command(arg_required_else_help = true)]
    Destroy {
        /// Identifier for the service instance
        #[arg(required = true)]
        instance_id: ServiceInstanceId,

        /// Retain state of the service. That state will no longer be managed by the service.
        #[arg(short, long)]
        retain: Option<bool>,
    },

    /// Bind a service
    #[command(arg_required_else_help = true)]
    Bind {
        /// Identifier for the service binding
        #[arg(long)]
        binding_id: Option<ServiceBindingId>,

        /// Identifier for the service instance
        #[arg(required = true)]
        instance_id: ServiceInstanceId,

        /// Scopes for the binding.
        #[arg(short, long)]
        scopes: Option<Vec<Scope>>,
    },

    /// Unbind a service
    #[command(arg_required_else_help = true)]
    Unbind {
        /// Identifier for the service binding
        #[arg(required = true)]
        binding_id: ServiceBindingId,

        /// Identifier for the service instance
        #[arg(required = true)]
        instance_id: ServiceInstanceId,
    },

    /// List bindings
    ListBindings {
        /// Identifier for the service instance
        #[arg(required = true)]
        instance_id: ServiceInstanceId,
    },
}

impl From<String> for Credential {
    fn from(value: String) -> Self {
        let cred: Vec<&str> = value.splitn(2, "=").collect();
        Self {
            key: String::from(cred[0]),
            value: String::from(*cred.get(1).unwrap_or(&"")),
        }
    }
}

impl From<String> for Request {
    fn from(value: String) -> Self {
        let cred: Vec<&str> = value.splitn(2, "=").collect();
        Self {
            key: String::from(cred[0]),
            value: String::from(*cred.get(1).unwrap_or(&"")),
        }
    }
}

struct HostComponent;

impl Guest for HostComponent {
    fn run() -> Result<(), ()> {
        match Cli::parse_from(environment::get_arguments().iter()).command {
            Commands::Provision {
                instance_id,
                type_,
                tier,
                requests,
            } => {
                log(
                    Level::Info,
                    "host",
                    &format!("Provisioning service: {type_}"),
                );

                let instance_id = match instance_id {
                    Some(instance_id) => instance_id,
                    None => UuidIds::generate_instance_id().map_err(|e| {
                        log(
                            Level::Error,
                            "host",
                            &format!("Error generating instance id: {}", e),
                        )
                    })?,
                };

                lifecycle::provision(&instance_id, &type_, tier.as_deref(), requests.as_deref())
                    .map_err(|e| {
                        log(Level::Error, "host", &format!("Error provisioning: {}", e))
                    })?;

                println!("{}", instance_id);

                Ok(())
            }
            Commands::Update {
                instance_id,
                tier,
                requests,
            } => {
                log(
                    Level::Info,
                    "host",
                    &format!("Updating service {}", instance_id),
                );

                lifecycle::update(&instance_id, tier.as_deref(), requests.as_deref())
                    .map_err(|e| log(Level::Error, "host", &format!("Error updating: {}", e)))
            }
            Commands::Destroy {
                instance_id,
                retain,
            } => {
                log(
                    Level::Info,
                    "host",
                    &format!("Destroying service {}", instance_id),
                );

                lifecycle::destroy(&instance_id, retain)
                    .map_err(|e| log(Level::Error, "host", &format!("Error destroying: {}", e)))
            }
            Commands::Bind {
                binding_id,
                instance_id,
                scopes,
            } => {
                log(Level::Info, "host", &format!("Binding to {}", instance_id));

                let binding_id = match binding_id {
                    Some(binding_id) => binding_id,
                    None => UuidIds::generate_binding_id(&instance_id).map_err(|e| {
                        log(
                            Level::Error,
                            "host",
                            &format!("Error generating binding id: {}", e),
                        )
                    })?,
                };
                let scopes = scopes.as_deref();

                lifecycle::bind(&binding_id, &instance_id, scopes).map_err(|e| {
                    log(
                        Level::Error,
                        "host",
                        &format!("Error binding to {}: {}", instance_id, e),
                    );
                })?;

                println!("{}", binding_id);

                Ok(())
            }
            Commands::Unbind {
                binding_id,
                instance_id,
            } => {
                log(Level::Info, "host", &format!("Unbinding {}", binding_id));

                lifecycle::unbind(&binding_id, &instance_id).map_err(|e| {
                    log(Level::Error, "host", &format!("Error unbinding: {}", e));
                })
            }
            Commands::ListBindings { instance_id } => {
                log(
                    Level::Info,
                    "host",
                    &format!("List bindings for {}", instance_id),
                );

                let bindings = lifecycle::list_bindings(&instance_id).map_err(|e| {
                    log(Level::Error, "host", &format!("Error listing: {}", e));
                })?;
                for binding in bindings {
                    println!("{binding}");
                }

                Ok(())
            }
        }
    }
}

#[macro_export]
macro_rules! println {
    () => {
        wasi::cli::stdout::get_stdout().blocking_write_and_flush("\n".as_bytes())
            .expect("failed writing to stdout")
    };
    ($($arg:tt)*) => {{
        wasi::cli::stdout::get_stdout().blocking_write_and_flush((std::format!($($arg)*) + "\n").as_bytes())
            .expect("failed writing to stdout")
    }};
}

pub(crate) struct UuidIds {}

impl UuidIds {
    pub fn generate() -> String {
        // cribbed from the uuid crate
        let bytes: [u8; 16] = get_random_bytes(16)
            .try_into()
            .expect("unexpected number of bytes");
        let src = (u128::from_be_bytes(bytes) & 0xFFFFFFFFFFFF4FFFBFFFFFFFFFFFFFFF
            | 0x40008000000000000000)
            .to_be_bytes();

        // lowercase letters
        let lut: [u8; 16] = [
            b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd',
            b'e', b'f',
        ];
        let groups = [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)];
        let mut dst = [0; 36];

        let mut group_idx = 0;
        let mut i = 0;
        while group_idx < 5 {
            let (start, end) = groups[group_idx];
            let mut j = start;
            while j < end {
                let x = src[i];
                i += 1;

                dst[j] = lut[(x >> 4) as usize];
                dst[j + 1] = lut[(x & 0x0f) as usize];
                j += 2;
            }
            if group_idx < 4 {
                dst[end] = b'-';
            }
            group_idx += 1;
        }
        String::from_utf8(dst.into_iter().collect()).unwrap()
    }

    fn validate_instance_id(instance_id: &ServiceInstanceId) -> Result<(), Error> {
        match Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0]{12}$")
            .unwrap()
            .is_match(instance_id)
        {
            false => Err(Error::from(format!(
                "expected instance-id to be a uuid, got: {}",
                instance_id
            ))),
            true => Ok(()),
        }
    }

    fn generate_instance_id() -> Result<ServiceInstanceId, Error> {
        let uuid = Self::generate();
        let instance_id: ServiceInstanceId = format!("{}-000000000000", uuid[0..23].to_string());
        Ok(instance_id)
    }

    fn generate_binding_id(instance_id: &ServiceInstanceId) -> Result<ServiceBindingId, Error> {
        Self::validate_instance_id(&instance_id)?;

        let uuid = Self::generate();
        let binding_id: ServiceBindingId = format!(
            "{}-{}",
            instance_id[0..23].to_string(),
            uuid[24..].to_string()
        );
        Ok(binding_id)
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "lifecycle-host-cli",
    features: ["clocks-timezone"],
    generate_all
});

export!(HostComponent);
