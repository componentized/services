//! The filesystem operations of the ops, on the preopened directories of `wasi:filesystem`.

use crate::wasi::filesystem::preopens::get_directories;
use crate::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, DirectoryEntry, ErrorCode, OpenFlags, PathFlags,
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

/// Rename the file or directory at the path to the new path.
pub async fn rename(from_path: &str, to_path: &str) -> Result<(), String> {
    let (from_dir, from_relative) = resolve(from_path)?;
    let (to_dir, to_relative) = resolve(to_path)?;
    from_dir
        .rename_at(from_relative, &to_dir, to_relative)
        .await
        .map_err(|e| format!("{from_path}: {e:?}"))
}

/// Remove the file at the path.
pub async fn remove_file(path: &str) -> Result<(), String> {
    let (dir, relative) = resolve(path)?;
    dir.unlink_file_at(relative)
        .await
        .map_err(|e| format!("{path}: {e:?}"))
}

/// The names of the entries of the directory at the path.
pub async fn read_dir(path: &str) -> Result<Vec<String>, String> {
    let (dir, relative) = resolve(path)?;
    let entries = entries(&dir, &relative)
        .await
        .map_err(|e| format!("{path}: {e:?}"))?;
    Ok(entries.into_iter().map(|entry| entry.name).collect())
}

/// The entries of the directory at the path relative to the directory, following a symbolic link
/// to a directory.
async fn entries(dir: &Descriptor, relative: &str) -> Result<Vec<DirectoryEntry>, ErrorCode> {
    let child = dir
        .open_at(
            PathFlags::SYMLINK_FOLLOW,
            relative.to_string(),
            OpenFlags::DIRECTORY,
            DescriptorFlags::READ,
        )
        .await?;
    let (entries, result) = child.read_directory();
    let entries = entries.collect().await;
    result.await?;
    Ok(entries)
}
