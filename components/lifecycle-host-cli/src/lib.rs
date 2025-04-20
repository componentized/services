use clap::{command, Parser, Subcommand};
use componentized::services::lifecycle;
use componentized::services::types::{
    Credential, Request, Scope, ServiceBindingId, ServiceInstanceId, Tier,
};
use exports::wasi::cli::run::Guest;
use wasi::cli::environment;
use wasi::logging::logging::{log, Level};

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
        /// Context for execution
        #[arg(long)]
        context: Option<String>,

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
        /// Context for execution
        #[arg(long)]
        context: Option<String>,

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
                context,
                type_,
                tier,
                requests,
            } => {
                log(
                    Level::Info,
                    "host",
                    &format!("Provisioning service: {type_}"),
                );

                let ctx = context.map(|c| c.into_bytes()).unwrap_or(vec![]);

                let service_id =
                    lifecycle::provision(&ctx, &type_, tier.as_ref(), requests.as_deref())
                        .map_err(|e| {
                            log(Level::Error, "host", &format!("Error provisioning: {}", e))
                        })?;

                println!("{}", service_id);

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

                lifecycle::update(&instance_id, tier.as_ref(), requests.as_deref())
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
                context,
                instance_id,
                scopes,
            } => {
                log(Level::Info, "host", &format!("Binding to {}", instance_id));

                let ctx = context.map(|c| c.into_bytes()).unwrap_or(vec![]);
                let scopes = scopes.as_deref();

                let binding_id = lifecycle::bind(&ctx, &instance_id, scopes).map_err(|e| {
                    log(
                        Level::Error,
                        "host",
                        &format!("Error binding to {}: {}", instance_id, e),
                    );
                })?;

                println!("{}", binding_id);

                Ok(())
            }
            Commands::Unbind { binding_id } => {
                log(Level::Info, "host", &format!("Unbinding {}", binding_id));

                lifecycle::unbind(&binding_id).map_err(|e| {
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

wit_bindgen::generate!({
    path: "../../wit",
    world: "lifecycle-host-cli",
    generate_all
});

export!(HostComponent);
