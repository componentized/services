//! The filesystem operations of the lifecycle, on the preopened directories of `wasi:filesystem`.

use crate::wasi::filesystem::preopens::get_directories;
use crate::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, DescriptorType, DirectoryEntry, ErrorCode, OpenFlags, PathFlags,
};

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

/// Remove the file at the path.
pub async fn remove_file(path: &str) -> Result<(), String> {
    let (dir, relative) = resolve(path)?;
    dir.unlink_file_at(relative)
        .await
        .map_err(|e| format!("{path}: {e:?}"))
}

/// Remove the empty directory at the path.
pub async fn remove_dir(path: &str) -> Result<(), String> {
    let (dir, relative) = resolve(path)?;
    dir.remove_directory_at(relative)
        .await
        .map_err(|e| format!("{path}: {e:?}"))
}

/// Remove the directory at the path, after removing its contents. Symbolic links are removed,
/// not followed.
pub async fn remove_dir_all(path: &str) -> Result<(), String> {
    let (dir, relative) = resolve(path)?;
    remove_contents(&dir, &relative)
        .await
        .map_err(|e| format!("{path}: {e:?}"))?;
    dir.remove_directory_at(relative)
        .await
        .map_err(|e| format!("{path}: {e:?}"))
}

/// Remove the contents of the directory at the path relative to the directory.
async fn remove_contents(dir: &Descriptor, relative: &str) -> Result<(), ErrorCode> {
    for entry in entries(dir, relative).await? {
        let path = format!("{relative}/{}", entry.name);
        match entry.type_ {
            DescriptorType::Directory => {
                Box::pin(remove_contents(dir, &path)).await?;
                dir.remove_directory_at(path).await?;
            }
            _ => dir.unlink_file_at(path).await?,
        }
    }
    Ok(())
}

/// Create a symbolic link at the path to the target, the target is not resolved.
pub async fn symlink(target: &str, path: &str) -> Result<(), String> {
    let (dir, relative) = resolve(path)?;
    dir.symlink_at(target.to_string(), relative)
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
