use clap::{Args, Parser, Subcommand};
use componentized::services::credential_admin::{destroy, publish};
use componentized::services::credential_store::fetch;
use componentized::services::lifecycle;
use componentized::services::types::{
    Credential, Error, Request, Scope, ServiceBindingId, ServiceId, ServiceInstanceId, Tier,
};
use componentized::services_test_components::ops;
use exports::wasi::cli::run::Guest;
use regex_lite::Regex;
use wasi::cli::environment::get_arguments;
use wasi::cli::{stderr, stdout};
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
    /// Publish credentials for a service instance or binding
    #[command(arg_required_else_help = true)]
    Publish {
        /// Identifier for the service instance or binding
        #[arg(required = true)]
        id: ServiceId,

        /// Credentials to publish key=value
        #[arg(short, long)]
        credentials: Vec<Credential>,
    },

    /// Destroy credentials for a service instance or binding
    #[command(arg_required_else_help = true)]
    Destroy {
        /// Identifier for the service instance or binding
        #[arg(required = true)]
        id: ServiceId,
    },

    /// Fetch credentials for a service instance or binding
    #[command(arg_required_else_help = true)]
    Fetch {
        /// Identifier for the service instance or binding
        #[arg(required = true)]
        id: ServiceId,
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

struct ServiceCli;

impl Guest for ServiceCli {
    async fn run() -> Result<(), ()> {
        let cli = match Cli::try_parse_from(get_arguments()) {
            Ok(cli) => cli,
            // help and version are errors that exit successfully, printed to stdout
            Err(e) => {
                let message = e.render().to_string().into_bytes();
                if e.use_stderr() {
                    write_stderr(message).await;
                } else {
                    write_stdout(message).await;
                }
                return if e.exit_code() == 0 { Ok(()) } else { Err(()) };
            }
        };
        match command(cli.command).await {
            Ok(()) => Ok(()),
            Err(message) => {
                eprintln(message).await;
                Err(())
            }
        }
    }
}

/// Write the data to the stdout stream.
async fn write_stdout(data: Vec<u8>) {
    let (mut writer, reader) = wit_stream::new();
    let result = stdout::write_via_stream(reader);
    writer.write_all(data).await;
    drop(writer);
    // nothing reports an error writing to stdout
    let _ = result.await;
}

/// Write the data to the stderr stream.
async fn write_stderr(data: Vec<u8>) {
    let (mut writer, reader) = wit_stream::new();
    let result = stderr::write_via_stream(reader);
    writer.write_all(data).await;
    drop(writer);
    // nothing reports an error writing to stderr
    let _ = result.await;
}

/// Write the line to stdout.
async fn println(line: impl Into<String>) {
    write_stdout((line.into() + "\n").into_bytes()).await
}

/// Write the line to stderr.
async fn eprintln(line: impl Into<String>) {
    write_stderr((line.into() + "\n").into_bytes()).await
}

/// Run the command, an error is the message to print to stderr.
async fn command(command: Commands) -> Result<(), String> {
    match command {
        Commands::Provision {
            type_,
            tier,
            requests,
        } => {
            eprintln(format!("Provisioning service: {type_}")).await;

            let instance_id = UuidIds::generate_instance_id()
                .map_err(|e| format!("Error generating instance id: {e}"))?;

            lifecycle::provision(instance_id.clone(), type_, tier, requests)
                .await
                .map_err(|e| format!("Error provisioning: {e}"))?;

            println(instance_id).await;

            Ok(())
        }
        Commands::Update {
            instance_id,
            tier,
            requests,
        } => {
            eprintln(format!("Updating service {instance_id}")).await;

            lifecycle::update(instance_id, tier, requests)
                .await
                .map_err(|e| format!("Error updating: {e}"))
        }
        Commands::Destroy {
            instance_id,
            retain,
        } => {
            eprintln(format!("Destroying service {instance_id}")).await;

            lifecycle::destroy(instance_id, retain)
                .await
                .map_err(|e| format!("Error destroying: {e}"))
        }
        Commands::Bind {
            instance_id,
            scopes,
        } => {
            eprintln(format!("Binding to {instance_id}")).await;

            let binding_id = UuidIds::generate_binding_id(&instance_id)
                .map_err(|e| format!("Error generating binding id: {e}"))?;

            lifecycle::bind(binding_id.clone(), instance_id.clone(), scopes)
                .await
                .map_err(|e| format!("Error binding to {instance_id}: {e}"))?;

            println(binding_id).await;

            Ok(())
        }
        Commands::Unbind {
            binding_id,
            instance_id,
        } => {
            eprintln(format!("Unbinding {binding_id}")).await;

            lifecycle::unbind(binding_id, instance_id)
                .await
                .map_err(|e| format!("Error unbinding: {e}"))
        }
        Commands::ListBindings { instance_id } => {
            eprintln(format!("List bindings for {instance_id}")).await;

            let bindings = lifecycle::list_bindings(instance_id)
                .await
                .map_err(|e| format!("Error listing: {e}"))?;
            for binding in bindings {
                println(binding).await;
            }

            Ok(())
        }
        Commands::Credentials(store) => match store.command {
            CredentialCommands::Publish { id, credentials } => {
                eprintln(format!("Publish creds for {id}")).await;

                publish(id.clone(), credentials)
                    .await
                    .map_err(|e| format!("Error publishing {id}: {e}"))
            }
            CredentialCommands::Destroy { id } => {
                eprintln(format!("Destroy creds for {id}")).await;

                destroy(id.clone())
                    .await
                    .map_err(|e| format!("Error destroying {id}: {e}"))
            }
            CredentialCommands::Fetch { id } => {
                eprintln(format!("Fetch creds for {id}")).await;

                let creds = fetch(id.clone())
                    .await
                    .map_err(|e| format!("Error fetching {id}: {e}"))?;
                println(format!("{creds:#?}")).await;

                Ok(())
            }
        },
        Commands::Ops(ops) => match ops.command {
            OpsCommands::List { path } => {
                eprintln(format!("List in {path}")).await;

                let mut path = path;
                if path == "/" || path == "." {
                    path = "".to_string();
                }

                let items = ops::list(path.clone())
                    .await
                    .map_err(|e| format!("Error listing {path}: {e}"))?;
                for item in items {
                    println(format!("  {item}")).await;
                }

                Ok(())
            }
            OpsCommands::Read { path } => {
                eprintln(format!("Reading {path}:")).await;

                let data = ops::read(path.clone())
                    .await
                    .map_err(|e| format!("Error reading {path}: {e}"))?;
                println(String::from_utf8_lossy(&data)).await;

                Ok(())
            }
            OpsCommands::Write { path, value } => {
                let data: Vec<u8> = value.into_bytes();
                // TODO get data from stdin

                ops::write(path.clone(), data)
                    .await
                    .map_err(|e| format!("Error writing {path}: {e}"))?;

                eprintln(format!("Wrote {path}")).await;

                Ok(())
            }
            OpsCommands::Move { from_path, to_path } => {
                eprintln(format!("Move {from_path} to {to_path}")).await;

                ops::move_(from_path, to_path)
                    .await
                    .map_err(|e| format!("Error moving: {e}"))
            }
            OpsCommands::Delete { path } => {
                eprintln(format!("Deleting {path}")).await;

                ops::delete(path.clone())
                    .await
                    .map_err(|e| format!("Error deleting {path}: {e}"))
            }
        },
    }
}

wit_bindgen::generate!({
    path: "../../tests/wit",
    world: "cli",
    generate_all
});

export!(ServiceCli);
