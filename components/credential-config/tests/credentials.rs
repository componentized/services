//! Tests for the `credential-config` component's exported `componentized:secrets/store`, the
//! credentials of the binding named by the imported config, fetched from an async credential
//! store.

use std::time::Duration;

use test_harness::{Fetch, Harness, Secret, SecretsError, TestSubject};

const BINDING_ID: &str = "binding-1";

async fn subject(fetch: Fetch) -> wasmtime::Result<TestSubject> {
    Harness::new("credential-config")
        .config("binding-id", BINDING_ID)
        .credentials(BINDING_ID, &[("username", "alice"), ("password", "s3cret")])
        .fetch(fetch)
        .build()
        .await
}

fn string(value: &str) -> Secret {
    Secret::String(value.to_string())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_returns_a_credential() -> wasmtime::Result<()> {
    let mut subject = subject(Fetch::Ready).await?;
    subject.secrets_init().await?.expect("init");
    assert_eq!(
        subject.secrets_get("username").await?.expect("get"),
        string("alice")
    );
    assert_eq!(
        subject.secrets_get("password").await?.expect("get"),
        string("s3cret")
    );
    assert_eq!(subject.fetched(), [BINDING_ID]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_a_missing_credential_is_not_found() -> wasmtime::Result<()> {
    let mut subject = subject(Fetch::Ready).await?;
    subject.secrets_init().await?.expect("init");
    assert_eq!(
        subject.secrets_get("missing").await?,
        Err(SecretsError::NotFound)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_before_init_is_not_initialized() -> wasmtime::Result<()> {
    let mut subject = subject(Fetch::Ready).await?;
    assert_eq!(
        subject.secrets_get("username").await?,
        Err(SecretsError::NotInitialized)
    );
    assert!(subject.fetched().is_empty());
    Ok(())
}

/// The credentials are fetched once, by the first `init`, later calls are no-ops.
#[tokio::test(flavor = "multi_thread")]
async fn init_again_is_a_no_op() -> wasmtime::Result<()> {
    let mut subject = subject(Fetch::Ready).await?;
    subject.secrets_init().await?.expect("init");
    subject.secrets_init().await?.expect("init again");
    assert_eq!(
        subject.secrets_get("username").await?.expect("get"),
        string("alice")
    );
    assert_eq!(subject.fetched(), [BINDING_ID]);
    Ok(())
}

/// The store's `fetch` is pending when the call returns, `init` waits for it.
#[tokio::test(flavor = "multi_thread")]
async fn init_waits_for_a_pending_fetch() -> wasmtime::Result<()> {
    let mut subject = subject(Fetch::Delayed(Duration::from_millis(10))).await?;
    subject.secrets_init().await?.expect("init");
    assert_eq!(
        subject.secrets_get("username").await?.expect("get"),
        string("alice")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn init_without_a_binding_id_is_an_error() -> wasmtime::Result<()> {
    let mut subject = Harness::new("credential-config").build().await?;
    assert_eq!(
        subject.secrets_init().await?,
        Err(SecretsError::Other(Some(
            "config: must contain 'binding-id'".to_string()
        )))
    );
    assert!(subject.fetched().is_empty());
    // a failed init leaves the store uninitialized
    assert_eq!(
        subject.secrets_get("username").await?,
        Err(SecretsError::NotInitialized)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn init_forwards_a_store_error() -> wasmtime::Result<()> {
    let mut subject = Harness::new("credential-config")
        .config("binding-id", "unknown")
        .build()
        .await?;
    assert_eq!(
        subject.secrets_init().await?,
        Err(SecretsError::Other(Some(
            "credential-store: no credentials for 'unknown'".to_string()
        )))
    );
    // the error is kept, a later init returns it without fetching again
    assert_eq!(
        subject.secrets_init().await?,
        Err(SecretsError::Other(Some(
            "credential-store: no credentials for 'unknown'".to_string()
        )))
    );
    assert_eq!(subject.fetched(), ["unknown"]);
    Ok(())
}
