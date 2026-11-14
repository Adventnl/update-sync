use crate::{
    model::OwnedFile,
    store::{is_link, safe_child},
};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;

const MAX_EXPANDED: u64 = 8 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 100_000;

pub fn unpack(package: &Path, name: &str, destination: &Path) -> Result<()> {
    let lower = name.to_lowercase();
    if lower.ends_with(".zip") {
        extract_zip(package, destination)?;
    } else if lower.ends_with(".tar.gz") {
        extract_tar(package, destination)?;
    } else if lower.ends_with(".dmg") {
        crate::platform::extract_dmg(package, destination)?;
    } else if lower.ends_with(".exe") || lower.ends_with(".jar") {
        anyhow::ensure!(!lower.contains("setup") && !lower.contains("install"), "This is a system installer. Publish a portable package to enable managed updates and removal.");
        let path = archive_path(destination, name)?;
        fs::copy(package, path)?;
    } else {
        anyhow::bail!("This package format is not supported. Use a portable ZIP, TAR.GZ, EXE, JAR, or macOS DMG.");
    }
    Ok(())
}

pub fn unpack_application(package: &Path, name: &str, destination: &Path) -> Result<Vec<String>> {
    unpack(package, name, destination)?;
    let mut candidates = entries(destination)?;
    if !candidates.is_empty() {
        return Ok(candidates);
    }
    let nested: Vec<_> = WalkDir::new(destination)
        .follow_links(false)
        .max_depth(3)
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            e.file_type().is_file()
                && (name.ends_with(".zip") || name.ends_with(".tar.gz") || name.ends_with(".dmg"))
        })
        .collect();
    if nested.len() == 1 {
        let temp = tempfile::tempdir_in(
            destination
                .parent()
                .context("The staging folder is missing.")?,
        )?;
        let content = temp.path().join("content");
        fs::create_dir(&content)?;
        unpack(
            nested[0].path(),
            &nested[0].file_name().to_string_lossy(),
            &content,
        )?;
        candidates = entries(&content)?;
        if !candidates.is_empty() {
            fs::remove_dir_all(destination)?;
            fs::rename(content, destination)?;
        }
    }
    Ok(candidates)
}

fn archive_path(root: &Path, name: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        !name.contains('\\') && !name.contains(':') && !name.contains('\0'),
        "An archive contains an unsafe path."
    );
    anyhow::ensure!(
        !name.split('/').any(|p| p == ".relay-owned"
            || p.ends_with('.')
            || p.ends_with(' ')
            || windows_reserved(p)),
        "An archive contains a reserved file name."
    );
    safe_child(root, name.trim_end_matches('/'))
}

fn windows_reserved(part: &str) -> bool {
    let stem = part
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

pub fn extract_zip(package: &Path, destination: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(package)?)?;
    anyhow::ensure!(
        zip.len() <= MAX_FILES,
        "The archive contains too many files."
    );
    let mut expanded = 0u64;
    let mut links = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        expanded = expanded
            .checked_add(entry.size())
            .context("Archive size overflow.")?;
        anyhow::ensure!(
            expanded <= MAX_EXPANDED,
            "The archive exceeds the 8 GB expanded size limit."
        );
        let path = archive_path(destination, entry.name())?;
        let mode = entry.unix_mode().unwrap_or(0o644);
        if mode & 0o170000 == 0o120000 {
            let mut target = String::new();
            entry.take(4096).read_to_string(&mut target)?;
            links.push((path, target));
            continue;
        }
        if entry.is_dir() {
            fs::create_dir_all(&path)?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .context("An archive contains duplicate paths.")?;
            std::io::copy(&mut entry, &mut out)?;
            out.flush()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(mode & 0o777))?;
            }
        }
    }
    create_links(destination, links)
}

fn extract_tar(package: &Path, destination: &Path) -> Result<()> {
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(fs::File::open(package)?));
    let mut expanded = 0u64;
    let mut links = Vec::new();
    for (index, entry) in tar.entries()?.enumerate() {
        anyhow::ensure!(index < MAX_FILES, "The archive contains too many files.");
        let mut entry = entry?;
        let raw = entry.path()?.to_string_lossy().into_owned();
        let name = raw.strip_prefix("./").unwrap_or(&raw).to_owned();
        if name.is_empty() || name == "." {
            continue;
        }
        let path = archive_path(destination, &name)?;
        expanded = expanded
            .checked_add(entry.size())
            .context("Archive size overflow.")?;
        anyhow::ensure!(
            expanded <= MAX_EXPANDED,
            "The archive exceeds the 8 GB expanded size limit."
        );
        let kind = entry.header().entry_type();
        if kind.is_symlink() {
            links.push((
                path,
                entry
                    .link_name()?
                    .context("Missing link target.")?
                    .to_string_lossy()
                    .into_owned(),
            ));
        } else if kind.is_dir() {
            fs::create_dir_all(path)?;
        } else if kind.is_file() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            std::io::copy(&mut entry, &mut out)?;
            out.flush()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    &path,
                    fs::Permissions::from_mode(entry.header().mode()? & 0o777),
                )?;
            }
        } else {
            anyhow::bail!(
                "The archive contains a hard link, device, or another unsupported file type."
            );
        }
    }
    create_links(destination, links)
}

fn create_links(root: &Path, links: Vec<(PathBuf, String)>) -> Result<()> {
    for (path, target) in &links {
        anyhow::ensure!(
            cfg!(target_os = "macos"),
            "This archive contains symbolic links unsupported on this platform."
        );
        let target = Path::new(target);
        anyhow::ensure!(
            !target.is_absolute()
                && !target
                    .components()
                    .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir)),
            "An archive link points outside its package."
        );
        let mut resolved = path.parent().context("Invalid archive link.")?.to_owned();
        for part in target.components() {
            match part {
                Component::ParentDir => {
                    resolved.pop();
                }
                Component::Normal(s) => resolved.push(s),
                Component::CurDir => {}
                _ => anyhow::bail!("Unsafe link."),
            }
            anyhow::ensure!(
                resolved.starts_with(root),
                "An archive link escapes its package."
            );
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        #[cfg(target_os = "macos")]
        std::os::unix::fs::symlink(target, path)?;
    }
    // Chained links must resolve inside the package too, with no dangling links.
    for (path, _) in links {
        anyhow::ensure!(
            path.canonicalize()?.starts_with(root.canonicalize()?),
            "An archive link escapes its package."
        );
    }
    Ok(())
}

pub fn inventory(root: &Path) -> Result<Vec<OwnedFile>> {
    let canonical = root.canonicalize()?;
    let mut files = Vec::new();
    let mut size = 0u64;
    for entry in WalkDir::new(root).follow_links(false).max_depth(64) {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if entry.depth() == 0 || metadata.is_dir() {
            continue;
        }
        anyhow::ensure!(files.len() < MAX_FILES, "Too many package files.");
        let relative = entry
            .path()
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if relative == ".relay-owned" {
            continue;
        }
        let (sha256, length) = if is_link(&metadata) {
            anyhow::ensure!(
                entry.path().canonicalize()?.starts_with(&canonical),
                "A package link points outside its folder."
            );
            (
                format!("link:{}", fs::read_link(entry.path())?.to_string_lossy()),
                0,
            )
        } else {
            size = size
                .checked_add(metadata.len())
                .context("Package size overflow.")?;
            anyhow::ensure!(size <= MAX_EXPANDED, "The expanded package exceeds 8 GB.");
            (hash_file(entry.path())?, metadata.len())
        };
        files.push(OwnedFile {
            path: relative,
            sha256,
            size: length,
        });
    }
    Ok(files)
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::default();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}

pub fn entries(root: &Path) -> Result<Vec<String>> {
    let mut candidates = Vec::new();
    for entry in WalkDir::new(root).follow_links(false).max_depth(12) {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let path = entry.path();
        if cfg!(target_os = "macos")
            && entry.file_type().is_dir()
            && name.ends_with(".app")
            && !path
                .ancestors()
                .skip(1)
                .take_while(|p| *p != root)
                .any(|p| p.extension().is_some_and(|e| e == "app"))
        {
            candidates.push(
                path.strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
        if entry.file_type().is_file()
            && ((cfg!(windows) && name.ends_with(".exe")) || name.ends_with(".jar"))
            && ![
                "unins",
                "setup",
                "install",
                "crash",
                "helper",
                "notification",
            ]
            .iter()
            .any(|p| name.contains(p))
        {
            candidates.push(
                path.strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    candidates.sort();
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_zip(path: &Path, name: &str) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"test").unwrap();
        zip.finish().unwrap();
    }
    #[test]
    fn zip_slip_and_reserved_paths_are_rejected() {
        let d = tempfile::tempdir().unwrap();
        let output = d.path().join("out");
        fs::create_dir(&output).unwrap();
        for name in [
            "../escaped",
            "C:/escaped",
            "a\\escaped",
            ".relay-owned",
            "NUL.txt",
        ] {
            let zip = d.path().join("test.zip");
            make_zip(&zip, name);
            assert!(extract_zip(&zip, &output).is_err(), "{name}");
        }
        assert!(!d.path().join("escaped").exists());
    }
    #[test]
    fn inventory_detects_changes() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("app.exe"), b"one").unwrap();
        let a = inventory(d.path()).unwrap();
        fs::write(d.path().join("app.exe"), b"two").unwrap();
        assert_ne!(a[0].sha256, inventory(d.path()).unwrap()[0].sha256);
    }
    #[test]
    fn one_nested_actions_package_is_unwrapped_without_running_it() {
        let d = tempfile::tempdir().unwrap();
        let inner = d.path().join("app.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&inner).unwrap());
        zip.start_file("Example.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"fixture").unwrap();
        zip.add_directory("Example.app/", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.finish().unwrap();
        let outer = d.path().join("artifact.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&outer).unwrap());
        zip.start_file("app.zip", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&fs::read(inner).unwrap()).unwrap();
        zip.finish().unwrap();
        let output = d.path().join("out");
        fs::create_dir(&output).unwrap();
        assert!(!unpack_application(&outer, "artifact.zip", &output)
            .unwrap()
            .is_empty());
        assert!(output.join("Example.exe").exists());
        assert!(!output.join("app.zip").exists());
    }
}
