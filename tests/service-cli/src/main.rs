use clap::{command, Args, Parser, Subcommand};
use componentized::services::credential_admin::{destroy, publish};
use componentized::services::credential_store::fetch;
use componentized::services::lifecycle;
use componentized::services::types::{
    Credential, Error, Request, Scope, ServiceBindingId, ServiceInstanceId, Tier,
};
use componentized::services_test_components::ops;
use regex_lite::Regex;
use std::io::{self, Write};
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

    /// Commands to interact with the credential store
    #[command(arg_required_else_help = true)]
    Credentials(CredentialsArgs),

    /// Commands to interact with the service
    #[command(arg_required_else_help = true)]
    Ops(OpsArgs),
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true)]
struct CredentialsArgs {
    #[command(subcommand)]
    command: CredentialCommands,
}

#[derive(Debug, Subcommand, Clone)]
enum CredentialCommands {
    /// Publish credentials for a binding
    #[command(arg_required_else_help = true)]
    Publish {
        /// Identifier for the service binding
        #[arg(required = true)]
        binding_id: ServiceBindingId,

        /// Credentials to publish key=value
        #[arg(short, long)]
        credentials: Vec<Credential>,
    },

    /// Destroy credentials for a binding
    #[command(arg_required_else_help = true)]
    Destroy {
        /// Identifier for the service binding
        #[arg(required = true)]
        binding_id: ServiceBindingId,
    },

    /// Fetch credentials for a binding
    #[command(arg_required_else_help = true)]
    Fetch {
        /// Identifier for the service binding
        #[arg(required = true)]
        binding_id: ServiceBindingId,
    },

    /// Export credentials for a binding as a wasi:config/store component
    #[command(arg_required_else_help = true)]
    Export {
        /// Identifier for the service binding
        #[arg(required = true)]
        binding_id: ServiceBindingId,
    },
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true)]
struct OpsArgs {
    #[command(subcommand)]
    command: OpsCommands,
}

#[derive(Debug, Subcommand, Clone)]
enum OpsCommands {
    #[command(arg_required_else_help = true)]
    List {
        /// Path to list
        #[arg(required = true)]
        path: String,
    },
    #[command(arg_required_else_help = true)]
    Read {
        /// Path to read
        #[arg(required = true)]
        path: String,
    },
    #[command(arg_required_else_help = true)]
    Write {
        /// Path to write
        #[arg(required = true)]
        path: String,

        /// Value to write
        #[arg(required = true)]
        value: String,
    },
    #[command(arg_required_else_help = true)]
    Move {
        /// Path to move
        #[arg(required = true)]
        from_path: String,

        /// Destination path
        #[arg(required = true)]
        to_path: String,
    },
    #[command(arg_required_else_help = true)]
    Delete {
        /// Path to delete
        #[arg(required = true)]
        path: String,
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

fn main() -> Result<(), ()> {
    match Cli::parse().command {
        Commands::Provision {
            type_,
            tier,
            requests,
        } => {
            eprintln!("Provisioning service: {type_}");

            let instance_id = UuidIds::generate_instance_id()
                .map_err(|e| eprintln!("Error generating instance id: {}", e))?;

            lifecycle::provision(&instance_id, &type_, tier.as_deref(), requests.as_deref())
                .map_err(|e| eprintln!("Error provisioning: {}", e))?;

            println!("{}", instance_id);

            Ok(())
        }
        Commands::Update {
            instance_id,
            tier,
            requests,
        } => {
            eprintln!("Updating service {}", instance_id);

            lifecycle::update(&instance_id, tier.as_deref(), requests.as_deref())
                .map_err(|e| eprintln!("Error updating: {}", e))
        }
        Commands::Destroy {
            instance_id,
            retain,
        } => {
            eprintln!("Destroying service {}", instance_id);

            lifecycle::destroy(&instance_id, retain)
                .map_err(|e| eprintln!("Error destroying: {}", e))
        }
        Commands::Bind {
            instance_id,
            scopes,
        } => {
            eprintln!("Binding to {}", instance_id);

            let binding_id = UuidIds::generate_binding_id(&instance_id).map_err(|e| {
                eprintln!("Error generating binding id: {}", e);
            })?;
            let scopes = scopes.as_deref();

            lifecycle::bind(&binding_id, &instance_id, scopes).map_err(|e| {
                eprintln!("Error binding to {}: {}", instance_id, e);
            })?;

            println!("{}", binding_id);

            Ok(())
        }
        Commands::Unbind {
            binding_id,
            instance_id,
        } => {
            eprintln!("Unbinding {}", binding_id);

            lifecycle::unbind(&binding_id, &instance_id).map_err(|e| {
                eprintln!("Error unbinding: {}", e);
            })
        }
        Commands::ListBindings { instance_id } => {
            eprintln!("List bindings for {}", instance_id);

            let bindings = lifecycle::list_bindings(&instance_id).map_err(|e| {
                eprintln!("Error listing: {}", e);
            })?;
            for binding in bindings {
                println!("{binding}");
            }

            Ok(())
        }
        Commands::Credentials(store) => match store.command {
            CredentialCommands::Publish {
                binding_id,
                credentials,
            } => {
                eprintln!("Publish creds for {}", binding_id);

                publish(&binding_id, &credentials).map_err(|e: Error| {
                    eprintln!("Error publishing {}: {}", binding_id, e);
                })
            }
            CredentialCommands::Destroy { binding_id } => {
                eprintln!("Destroy creds for {}", binding_id);

                destroy(&binding_id).map_err(|e: Error| {
                    eprintln!("Error destroying {}: {}", binding_id, e);
                })
            }
            CredentialCommands::Fetch { binding_id } => {
                eprintln!("Fetch creds for {}", binding_id);

                let creds = fetch(&binding_id).map_err(|e: Error| {
                    eprintln!("Error fetching {}: {}", binding_id, e);
                })?;
                println!("{:#?}", creds);

                Ok(())
            }
            CredentialCommands::Export { binding_id } => {
                eprintln!("Export creds for {}", binding_id);

                let creds: Vec<(String, String)> = fetch(&binding_id)
                    .map_err(|e: Error| {
                        eprintln!("Error fetching {}: {}", binding_id, e);
                    })?
                    .iter()
                    .map(|cred| (cred.key.clone(), cred.value.clone()))
                    .collect();
                let config =
                    componentized::config::factory::build_component(&creds).map_err(|e| {
                        eprintln!("Error creating config {}: {}", binding_id, e);
                    })?;

                let mut output = Box::new(io::stdout()) as Box<dyn Write>;
                output.write_all(&config).map_err(|e| {
                    eprintln!("Error writing config {}: {}", binding_id, e);
                })?;

                Ok(())
            }
        },
        Commands::Ops(ops) => match ops.command {
            OpsCommands::List { path } => {
                eprintln!("List in {}", path);

                let mut path = path;
                if path == "/" || path == "." {
                    path = "".to_string();
                }

                let items = ops::list(&path).map_err(|e: Error| {
                    eprintln!("Error listing {}: {}", path, e);
                })?;
                for item in items {
                    println!("  {}", item);
                }

                Ok(())
            }
            OpsCommands::Read { path } => {
                eprintln!("Reading {}:", path);

                let data = ops::read(&path).map_err(|e: Error| {
                    eprintln!("Error reading {}: {}", path, e);
                })?;
                println!("{}", String::from_utf8(data).unwrap());

                Ok(())
            }
            OpsCommands::Write { path, value } => {
                let data: Vec<u8> = value.as_bytes().to_vec();
                // TODO get data from stdin
                // io::stdin().read_to_end(&mut data).map_err(|e| {
                //     eprintln!("Error reading stdin: {}", e);
                // })?;

                ops::write(&path, data.as_slice()).map_err(|e| {
                    eprintln!("Error writing {}: {}", path, e);
                })?;

                eprintln!("Wrote {path}");

                Ok(())
            }
            OpsCommands::Move { from_path, to_path } => {
                eprintln!("Move {} to {}", from_path, to_path);

                ops::move_(&from_path, &to_path).map_err(|e: Error| {
                    eprintln!("Error moving: {}", e);
                })?;

                Ok(())
            }
            OpsCommands::Delete { path } => {
                eprintln!("Deleting {}", path);

                ops::delete(&path).map_err(|e: Error| {
                    eprintln!("Error deleting {}: {}", path, e);
                })?;

                Ok(())
            }
        },
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "cli",
    generate_all
});
