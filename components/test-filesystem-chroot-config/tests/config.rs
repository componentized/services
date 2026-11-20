//! Tests for the `test-filesystem-chroot-config` component's exported `wasi:config/store`, the `path`
//! secret of the imported `componentized:secrets/store`, initialized elsewhere.

use test_harness::{ConfigError, Harness, Secret, TestSubject};

const PATH: &str = "services/instances/i-1/bindings/b-1";

fn string(value: &str) -> Secret {
    Secret::String(value.to_string())
}

/// A subject whose secrets store was initialized elsewhere, with a `path` and another secret.
async fn subject() -> wasmtime::Result<TestSubject> {
    Harness::new("test-filesystem-chroot-config")
        .secret("path", string(PATH))
        .secret("password", string("s3cret"))
        .secrets_initialized(true)
        .build()
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn get_returns_the_path() -> wasmtime::Result<()> {
    let mut subject = subject().await?;
    assert_eq!(
        subject.config_get("path").await?.expect("get"),
        Some(PATH.to_string())
    );
    assert_eq!(subject.secrets_got(), ["path"]);
    Ok(())
}

/// Only `path` is exposed, other keys aren't looked up in the secrets store.
#[tokio::test(flavor = "multi_thread")]
async fn get_hides_other_secrets() -> wasmtime::Result<()> {
    let mut subject = subject().await?;
    assert_eq!(subject.config_get("password").await?.expect("get"), None);
    assert_eq!(subject.config_get("missing").await?.expect("get"), None);
    assert!(subject.secrets_got().is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_all_returns_only_the_path() -> wasmtime::Result<()> {
    let mut subject = subject().await?;
    assert_eq!(
        subject.config_get_all().await?.expect("get-all"),
        vec![("path".to_string(), PATH.to_string())]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_without_a_path_is_none() -> wasmtime::Result<()> {
    let mut subject = Harness::new("test-filesystem-chroot-config")
        .secret("password", string("s3cret"))
        .secrets_initialized(true)
        .build()
        .await?;
    assert_eq!(subject.config_get("path").await?.expect("get"), None);
    assert_eq!(subject.config_get_all().await?.expect("get-all"), vec![]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_a_bytes_path_returns_the_text() -> wasmtime::Result<()> {
    let mut subject = Harness::new("test-filesystem-chroot-config")
        .secret("path", Secret::Bytes(PATH.as_bytes().to_vec()))
        .secrets_initialized(true)
        .build()
        .await?;
    assert_eq!(
        subject.config_get("path").await?.expect("get"),
        Some(PATH.to_string())
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_a_bytes_path_that_is_not_text_is_an_error() -> wasmtime::Result<()> {
    let mut subject = Harness::new("test-filesystem-chroot-config")
        .secret("path", Secret::Bytes(vec![0xff, 0xfe]))
        .secrets_initialized(true)
        .build()
        .await?;
    match subject.config_get("path").await? {
        Err(ConfigError::Upstream(message)) => {
            assert_eq!(message, "secrets: 'path' is not utf-8 text")
        }
        other => panic!("expected an upstream error, got {other:?}"),
    }
    Ok(())
}

/// The config is sync, it never initializes the secrets store itself.
#[tokio::test(flavor = "multi_thread")]
async fn get_before_the_secrets_are_initialized_is_an_error() -> wasmtime::Result<()> {
    let mut subject = Harness::new("test-filesystem-chroot-config")
        .secret("path", string(PATH))
        .build()
        .await?;
    for result in [
        subject.config_get("path").await?.map(|_| ()),
        subject.config_get_all().await?.map(|_| ()),
    ] {
        match result {
            Err(ConfigError::Upstream(message)) => {
                assert_eq!(message, "secrets: the store is not initialized")
            }
            other => panic!("expected an upstream error, got {other:?}"),
        }
    }
    Ok(())
}
