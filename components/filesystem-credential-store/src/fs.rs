//! The filesystem operations of the store, on the preopened directories of `wasi:filesystem`.

use crate::wasi::filesystem::preopens::get_directories;
use crate::wasi::filesystem::types::{Descriptor, DescriptorFlags, OpenFlags, PathFlags};

/// The preopened directory the path is in, with the path relative to it, the preopen with the
/// longest name the path starts with. Like wasi-libc, the working directory is `/`, a relative
/// path is in the same preopen as the absolute path.
fn resolve(path: &str) -> Result<(Descriptor, String), String> {
    let components = |path: &str| -> Vec<String> {
        path.split('/')
            .filter(|c| !c.is_empty() && *c != ".")
            .map(String::from)
            .collect()
    };
    let path = components(path);
    get_directories()
        .into_iter()
        .map(|(dir, name)| (dir, components(&name)))
        .filter(|(_, name)| path.starts_with(name))
        .max_by_key(|(_, name)| name.len())
        .map(|(dir, name)| match path[name.len()..].join("/") {
            relative if relative.is_empty() => (dir, String::from(".")),
            relative => (dir, relative),
        })
        .ok_or_else(|| format!("{}: not in a preopened directory", path.join("/")))
}

/// Read the whole file at the path.
pub async fn read(path: &str) -> Result<Vec<u8>, String> {
    let (dir, relative) = resolve(path)?;
    let file = dir
        .open_at(
            PathFlags::SYMLINK_FOLLOW,
            relative,
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .await
        .map_err(|e| format!("{path}: {e:?}"))?;
    let (data, result) = file.read_via_stream(0);
    let data = data.collect().await;
    result.await.map_err(|e| format!("{path}: {e:?}"))?;
    Ok(data)
}
