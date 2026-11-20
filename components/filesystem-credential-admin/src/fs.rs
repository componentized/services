//! The filesystem operations of the credential admin, on the preopened directories of `wasi:filesystem`.

use crate::wasi::filesystem::preopens::get_directories;
use crate::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags,
};
use crate::wit_stream;

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

/// Create the directory at the path, with any parents that don't exist.
pub async fn create_dir_all(path: &str) -> Result<(), String> {
    let (dir, relative) = resolve(path)?;
    let mut parent = String::new();
    for component in relative.split('/').filter(|c| *c != ".") {
        if !parent.is_empty() {
            parent.push('/');
        }
        parent.push_str(component);
        match dir.create_directory_at(parent.clone()).await {
            Ok(()) | Err(ErrorCode::Exist) => {}
            Err(e) => return Err(format!("{path}: {e:?}")),
        }
    }
    Ok(())
}

/// Write the data to the file at the path, replacing the file's contents when it exists.
pub async fn write(path: &str, data: Vec<u8>) -> Result<(), String> {
    let (dir, relative) = resolve(path)?;
    let file = dir
        .open_at(
            PathFlags::SYMLINK_FOLLOW,
            relative,
            OpenFlags::CREATE | OpenFlags::TRUNCATE,
            DescriptorFlags::WRITE,
        )
        .await
        .map_err(|e| format!("{path}: {e:?}"))?;
    let (mut writer, reader) = wit_stream::new();
    let result = file.write_via_stream(reader, 0);
    writer.write_all(data).await;
    drop(writer);
    result.await.map_err(|e| format!("{path}: {e:?}"))
}

/// Remove the file at the path.
pub async fn remove_file(path: &str) -> Result<(), String> {
    let (dir, relative) = resolve(path)?;
    dir.unlink_file_at(relative)
        .await
        .map_err(|e| format!("{path}: {e:?}"))
}
