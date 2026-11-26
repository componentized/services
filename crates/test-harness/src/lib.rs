//! Test harness for the components.
//!
//! A [`Harness`] builds a component (`target/components/*/*.wasm`) with make and instantiates it
//! in wasmtime, with component model async and cooperative threads, with each import the component
//! needs provided by the host, e.g. `wasi:config/store` from the config the test gives it,
//! `componentized:services/credential-store` from the credentials the test gives it and
//! `componentized:secrets/store` from the secrets the test gives it. The subject's exported
//! `componentized:secrets/store` or `wasi:config/store` is called through the [`TestSubject`].

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use wasmtime::component::{Accessor, Component, HasData, HasSelf, Instance, Linker};
use wasmtime::error::Context as _;
use wasmtime::{Config, Engine, Result, Store, bail};

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "../../components/wit",
        inline: "
            package componentized:test-harness;

            world harness {
                import componentized:services/credential-store@0.0.6-dev;
                import wasi:config/store@0.2.0-rc.1;
                import componentized:secrets/store@0.1.1;
                export componentized:secrets/store@0.1.1;
                export wasi:config/store@0.2.0-rc.1;
            }
        ",
        world: "componentized:test-harness/harness",
        imports: {
            "componentized:services/credential-store": async | store,
        },
        exports: { default: async | store },
        additional_derives: [PartialEq],
    });
}

use bindings::componentized::secrets::store as secrets_import;
use bindings::componentized::services::credential_store;
use bindings::componentized::services::types::Credential;
use bindings::exports::componentized::secrets::store as secrets_export;
use bindings::exports::wasi::config::store as config_export;
use bindings::wasi::config::store as config_import;

pub use bindings::componentized::secrets::types::{ErrorCode as SecretsError, Secret};
pub use bindings::exports::wasi::config::store::Error as ConfigError;

/// Root of the workspace, where the Makefile lives.
fn workspace_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    dir.canonicalize().unwrap_or(dir)
}

/// Path of the built component.
pub fn component_path(name: &str) -> PathBuf {
    workspace_dir().join(format!("target/components/{name}/{name}.wasm"))
}

/// Build the named component in `target/components/` with make, unless it was already built by
/// this process.
fn ensure_built(name: &str) -> Result<()> {
    static BUILT: Mutex<Option<HashSet<String>>> = Mutex::new(None);

    // hold the lock while building so concurrent tests don't run make over each other
    let mut built = BUILT.lock().unwrap_or_else(|err| err.into_inner());
    let built = built.get_or_insert_with(HashSet::new);
    if built.contains(name) {
        return Ok(());
    }

    // the Makefile's targets are relative to the workspace, make matches them textually
    let target = format!("target/components/{name}/{name}.wasm");
    let output = Command::new("make")
        .arg("-C")
        .arg(workspace_dir())
        .arg(&target)
        .output()
        .context("failed to run make")?;
    if !output.status.success() {
        bail!(
            "failed to build {target}:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    built.insert(name.to_string());
    Ok(())
}

/// How the host's credential store answers `fetch`.
#[derive(Clone, Copy, Debug, Default)]
pub enum Fetch {
    /// Return before the guest waits, the subtask has returned by the time the call does.
    #[default]
    Ready,
    /// Return after the delay, the guest's subtask is pending and the guest waits for it.
    Delayed(Duration),
}

/// The host's side of the component's imports.
pub struct Ctx {
    config: HashMap<String, String>,
    credentials: HashMap<String, Vec<(String, String)>>,
    fetch: Fetch,
    fetched: Vec<String>,
    secrets: HashMap<String, Secret>,
    secrets_initialized: bool,
    secrets_got: Vec<String>,
}

impl config_import::Host for Ctx {
    fn get(&mut self, key: String) -> Result<Option<String>, config_import::Error> {
        Ok(self.config.get(&key).cloned())
    }

    fn get_all(&mut self) -> Result<Vec<(String, String)>, config_import::Error> {
        Ok(self
            .config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }
}

impl credential_store::Host for Ctx {}

/// The host's `componentized:secrets/store`, its `init` is async.
struct SecretsStore;

impl HasData for SecretsStore {
    type Data<'a> = &'a mut Ctx;
}

impl<T: 'static> secrets_import::HostWithStore<T> for SecretsStore {
    async fn init(accessor: &Accessor<T, Self>) -> Result<(), SecretsError> {
        accessor.with(|mut store| store.get().secrets_initialized = true);
        Ok(())
    }
}

impl secrets_import::Host for Ctx {
    fn get(&mut self, key: String) -> Result<Secret, SecretsError> {
        self.secrets_got.push(key.clone());
        if !self.secrets_initialized {
            return Err(SecretsError::NotInitialized);
        }
        self.secrets
            .get(&key)
            .cloned()
            .ok_or(SecretsError::NotFound)
    }
}

/// The host's `componentized:services/credential-store`, its `fetch` is async.
struct CredentialStore;

impl HasData for CredentialStore {
    type Data<'a> = &'a mut Ctx;
}

impl<T: 'static> credential_store::HostWithStore<T> for CredentialStore {
    async fn fetch(accessor: &Accessor<T, Self>, id: String) -> Result<Vec<Credential>, String> {
        let fetch = accessor.with(|mut store| {
            let ctx = store.get();
            ctx.fetched.push(id.clone());
            ctx.fetch
        });
        if let Fetch::Delayed(delay) = fetch {
            tokio::time::sleep(delay).await;
        }
        accessor.with(|mut store| {
            store
                .get()
                .credentials
                .get(&id)
                .map(|credentials| {
                    credentials
                        .iter()
                        .map(|(key, value)| Credential {
                            key: key.clone(),
                            value: value.clone(),
                        })
                        .collect()
                })
                .ok_or_else(|| format!("no credentials for '{id}'"))
        })
    }
}

/// A test subject, before it's instantiated.
pub struct Harness {
    subject: String,
    config: HashMap<String, String>,
    credentials: HashMap<String, Vec<(String, String)>>,
    fetch: Fetch,
    secrets: HashMap<String, Secret>,
    secrets_initialized: bool,
}

impl Harness {
    /// The component named, e.g. `credential-config` for
    /// `target/components/credential-config/credential-config.wasm`.
    pub fn new(component_name: &str) -> Self {
        Self {
            subject: component_name.to_string(),
            config: HashMap::new(),
            credentials: HashMap::new(),
            fetch: Fetch::default(),
            secrets: HashMap::new(),
            secrets_initialized: false,
        }
    }

    /// A secret of the imported `componentized:secrets/store`.
    pub fn secret(mut self, key: &str, secret: Secret) -> Self {
        self.secrets.insert(key.to_string(), secret);
        self
    }

    /// Whether the imported `componentized:secrets/store` starts initialized, as though `init` was
    /// called elsewhere. Otherwise it's initialized once the subject calls `init`.
    pub fn secrets_initialized(mut self, initialized: bool) -> Self {
        self.secrets_initialized = initialized;
        self
    }

    /// A value of the imported `wasi:config/store`.
    pub fn config(mut self, key: &str, value: &str) -> Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }

    /// The credentials the imported `componentized:services/credential-store` fetches for the id.
    pub fn credentials(mut self, id: &str, credentials: &[(&str, &str)]) -> Self {
        self.credentials.insert(
            id.to_string(),
            credentials
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        self
    }

    /// How the imported credential store answers `fetch`.
    pub fn fetch(mut self, fetch: Fetch) -> Self {
        self.fetch = fetch;
        self
    }

    /// Build and instantiate the test subject.
    pub async fn build(self) -> Result<TestSubject> {
        ensure_built(&self.subject)?;

        let mut config = Config::new();
        config.wasm_component_model_async(true);
        let engine = Engine::new(&config)?;

        let path = component_path(&self.subject);
        let component = Component::from_file(&engine, &path)
            .with_context(|| format!("failed to load {}", path.display()))?;

        let mut linker = Linker::new(&engine);
        config_import::add_to_linker::<_, HasSelf<Ctx>>(&mut linker, |ctx| ctx)?;
        credential_store::add_to_linker::<_, CredentialStore>(&mut linker, |ctx| ctx)?;
        secrets_import::add_to_linker::<_, SecretsStore>(&mut linker, |ctx| ctx)?;

        let mut store = Store::new(
            &engine,
            Ctx {
                config: self.config,
                credentials: self.credentials,
                fetch: self.fetch,
                fetched: Vec::new(),
                secrets: self.secrets,
                secrets_initialized: self.secrets_initialized,
                secrets_got: Vec::new(),
            },
        );
        let instance = linker
            .instantiate_async(&mut store, &component)
            .await
            .with_context(|| format!("failed to instantiate {}", self.subject))?;
        Ok(TestSubject { store, instance })
    }
}

/// An instantiated test subject.
pub struct TestSubject {
    store: Store<Ctx>,
    instance: Instance,
}

impl TestSubject {
    /// The ids the subject fetched credentials for, in order.
    pub fn fetched(&self) -> &[String] {
        &self.store.data().fetched
    }

    /// The keys the subject got from the imported secrets store, in order.
    pub fn secrets_got(&self) -> &[String] {
        &self.store.data().secrets_got
    }

    /// Call the subject's exported `componentized:secrets/store#init`.
    pub async fn secrets_init(&mut self) -> Result<Result<(), SecretsError>> {
        let store = self.secrets_store()?;
        self.store
            .run_concurrent(async move |accessor| store.call_init(accessor).await)
            .await?
    }

    /// Call the subject's exported `componentized:secrets/store#get`.
    pub async fn secrets_get(&mut self, key: &str) -> Result<Result<Secret, SecretsError>> {
        let store = self.secrets_store()?;
        let key = key.to_string();
        self.store
            .run_concurrent(async move |accessor| store.call_get(accessor, key).await)
            .await?
    }

    fn secrets_store(&mut self) -> Result<secrets_export::Guest> {
        let indices = secrets_export::GuestIndices::new(&self.instance.instance_pre(&self.store))?;
        indices.load(&mut self.store, &self.instance)
    }

    /// Call the subject's exported `wasi:config/store#get`.
    pub async fn config_get(&mut self, key: &str) -> Result<Result<Option<String>, ConfigError>> {
        let store = self.config_store()?;
        let key = key.to_string();
        self.store
            .run_concurrent(async move |accessor| store.call_get(accessor, key).await)
            .await?
    }

    /// Call the subject's exported `wasi:config/store#get-all`.
    pub async fn config_get_all(&mut self) -> Result<Result<Vec<(String, String)>, ConfigError>> {
        let store = self.config_store()?;
        self.store
            .run_concurrent(async move |accessor| store.call_get_all(accessor).await)
            .await?
    }

    fn config_store(&mut self) -> Result<config_export::Guest> {
        let indices = config_export::GuestIndices::new(&self.instance.instance_pre(&self.store))?;
        indices.load(&mut self.store, &self.instance)
    }
}
