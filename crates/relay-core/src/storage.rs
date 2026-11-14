use crate::{
    manager::Manager,
    platform,
    store::{is_link, safe_child},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path, time::SystemTime};
use walkdir::WalkDir;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub kind: String,
    pub app_id: Option<String>,
    pub detail: String,
}

fn size(path: &Path) -> Result<u64> {
    let mut bytes = 0u64;
    for entry in WalkDir::new(path).follow_links(false).max_depth(64) {
        let entry = entry?;
        if entry.file_type().is_file() {
            bytes = bytes.saturating_add(entry.metadata()?.len());
        }
    }
    Ok(bytes)
}
fn cache_stats(path: &Path) -> Result<(u64, SystemTime)> {
    let mut bytes = 0u64;
    let mut newest = SystemTime::UNIX_EPOCH;
    for entry in WalkDir::new(path).follow_links(false).max_depth(64) {
        let entry = entry?;
        let meta = entry.metadata()?;
        if entry.file_type().is_symlink() {
            continue;
        }
        newest = newest.max(meta.modified()?);
        if meta.is_file() {
            bytes = bytes.saturating_add(meta.len());
        }
    }
    Ok((bytes, newest))
}

impl Manager {
    pub fn storage(&self) -> Result<Vec<StorageItem>> {
        let store = self.store()?;
        let apps = store.apps()?;
        let mut items = Vec::new();
        for app in &apps {
            // Keep the active version and one most-recent alternate for rollback.
            let alternate = app
                .versions
                .iter()
                .find(|v| Some(&v.id) != app.active_version.as_ref())
                .map(|v| v.id.as_str());
            for v in &app.versions {
                if Some(&v.id) == app.active_version.as_ref() || Some(v.id.as_str()) == alternate {
                    continue;
                }
                let path = self.assert_owned_version(&store, &app.id, &v.id)?;
                if path.exists() {
                    items.push(StorageItem {
                        id: format!("version:{}:{}", app.id, v.id),
                        name: format!("{} {}", app.name, v.label),
                        path: path.to_string_lossy().into(),
                        size: size(&path)?,
                        kind: "version".into(),
                        app_id: Some(app.id.clone()),
                        detail:
                            "Older version. Your current version and one rollback version are kept."
                                .into(),
                    });
                }
            }
        }
        for area in ["staging", "builds"] {
            for entry in fs::read_dir(self.root.join(area))? {
                let entry = entry?;
                let path = entry.path();
                if is_link(&fs::symlink_metadata(&path)?) || !path.is_dir() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                let owner = fs::read_to_string(path.join(".relay-owned")).unwrap_or_default();
                if (area == "staging" && owner != name && owner != "download")
                    || (area == "builds" && owner != "build")
                    || path.join("promotion.json").exists()
                {
                    continue;
                }
                items.push(StorageItem {
                    id: format!("{area}:{name}"),
                    name: if area == "builds" {
                        "Interrupted local build"
                    } else {
                        "Prepared or interrupted download"
                    }
                    .into(),
                    path: path.to_string_lossy().into(),
                    size: size(&path)?,
                    kind: "staging".into(),
                    app_id: None,
                    detail: "A temporary package. Removing it does not change installed apps."
                        .into(),
                });
            }
        }
        let referenced: HashSet<_> = apps
            .iter()
            .flat_map(|a| a.versions.iter().map(|v| v.id.as_str()))
            .collect();
        for app in &apps {
            let versions = store.app_dir(&app.id)?.join("versions");
            if !versions.exists() {
                continue;
            }
            for entry in fs::read_dir(versions)? {
                let entry = entry?;
                let id = entry.file_name().to_string_lossy().into_owned();
                if referenced.contains(id.as_str()) || uuid::Uuid::parse_str(&id).is_err() {
                    continue;
                }
                if let Ok(path) = self.assert_owned_version(&store, &app.id, &id) {
                    items.push(StorageItem {
                        id: format!("orphan:{}:{id}", app.id),
                        name: format!("{} interrupted installation", app.name),
                        size: size(&path)?,
                        path: path.to_string_lossy().into(),
                        kind: "orphan".into(),
                        app_id: Some(app.id.clone()),
                        detail: "Verified store-owned files with no active installation record."
                            .into(),
                    });
                }
            }
        }
        Ok(items)
    }
    pub fn cleanup(&self, ids: &[String]) -> Result<u64> {
        let _guard = self.lock()?;
        // Recompute candidates under the process lock. UI paths are never trusted.
        let allowed: std::collections::HashMap<_, _> = self
            .storage()?
            .into_iter()
            .map(|i| (i.id.clone(), i))
            .collect();
        let mut store = self.store()?;
        let mut freed = 0;
        for id in ids {
            self.check_cancelled()?;
            let item = allowed
                .get(id)
                .context("Storage changed. Review the cleanup list again.")?;
            if let Some(app_id) = &item.app_id {
                anyhow::ensure!(
                    !platform::running(&store.app_dir(app_id)?),
                    "Close the application before cleaning its old versions."
                );
            }
            let path = Path::new(&item.path);
            anyhow::ensure!(
                path.starts_with(&self.root),
                "Cleanup path is outside Relay's storage."
            );
            let relative = path
                .strip_prefix(&self.root)?
                .to_str()
                .context("Invalid cleanup path.")?;
            let path = safe_child(&self.root, relative)?;
            fs::remove_dir_all(&path)?;
            if item.kind == "version" {
                let parts: Vec<_> = id.split(':').collect();
                let mut app = store.app(parts[1])?;
                app.versions.retain(|v| v.id != parts[2]);
                store.save(
                    &app,
                    "Storage cleaned",
                    "Removed an older, inactive version.",
                )?;
            }
            freed += item.size;
        }
        Ok(freed)
    }
    pub fn scan_caches(&self, root: &Path) -> Result<Vec<StorageItem>> {
        let root = root.canonicalize()?;
        anyhow::ensure!(
            root.is_dir() && root.parent().is_some(),
            "Choose a development projects folder, not a disk root."
        );
        anyhow::ensure!(
            !root.starts_with(&self.root),
            "Use the managed storage list for Relay's own folders."
        );
        let mut items = Vec::new();
        let mut walker = WalkDir::new(&root)
            .follow_links(false)
            .max_depth(5)
            .into_iter();
        let mut count = 0;
        while let Some(entry) = walker.next() {
            count += 1;
            self.check_cancelled()?;
            anyhow::ensure!(
                count < 30_000,
                "This folder contains too many entries. Choose a smaller projects folder."
            );
            let entry = entry?;
            if is_link(&fs::symlink_metadata(entry.path())?)
                || [".git", "node_modules", ".venv"]
                    .contains(&entry.file_name().to_string_lossy().as_ref())
            {
                walker.skip_current_dir();
                continue;
            }
            if !entry.file_type().is_dir() {
                continue;
            }
            let parent = entry.path().parent().context("Cache has no parent.")?;
            let name = entry.file_name().to_string_lossy();
            let kind = if name == "target" && parent.join("Cargo.toml").is_file() {
                Some("Rust build cache")
            } else if name == "build"
                && parent.join("CMakeLists.txt").is_file()
                && entry.path().join("CMakeCache.txt").is_file()
            {
                Some("CMake build cache")
            } else {
                None
            };
            if let Some(kind) = kind {
                walker.skip_current_dir();
                let (bytes, modified) = cache_stats(entry.path())?;
                // A recent cache may still be in use. Always check again before deletion.
                if modified.elapsed().unwrap_or_default().as_secs() < 24 * 3600 {
                    continue;
                }
                let path = entry.path().canonicalize()?;
                items.push(StorageItem {id:format!("cache:{}",path.display()),name:parent.file_name().unwrap_or_default().to_string_lossy().into(),path:path.to_string_lossy().into(),size:bytes,kind:"cache".into(),app_id:None,detail:format!("{kind}. Every file is older than a day. Rebuilt by your toolchain when needed.")});
            }
        }
        Ok(items)
    }
    pub fn remove_cache(&self, root: &Path, path: &Path) -> Result<u64> {
        let _guard = self.lock()?;
        let root = root.canonicalize()?;
        let path = path.canonicalize()?;
        let candidate = self
            .scan_caches(&root)?
            .into_iter()
            .find(|i| Path::new(&i.path) == path)
            .context("This is no longer a safe cleanup candidate. Scan again.")?;
        let relative = path
            .strip_prefix(&root)?
            .to_str()
            .context("Invalid cache path.")?;
        safe_child(&root, relative)?;
        anyhow::ensure!(
            !platform::running(&path),
            "This cache contains a running program. Close it before cleanup."
        );
        // Cargo/CMake can be writing without a process executable inside the cache.
        let mut system = sysinfo::System::new_all();
        system.refresh_all();
        anyhow::ensure!(!system.processes().values().any(|p|["cargo","cargo.exe","rustc","rustc.exe","cmake","cmake.exe","ninja","ninja.exe","msbuild.exe","cl.exe","link.exe"].contains(&p.name().to_string_lossy().to_lowercase().as_str())),"A build tool is running. Wait for builds to finish before removing development caches.");
        anyhow::ensure!(!system.processes().values().any(|p|p.cwd().is_some_and(|p|p.starts_with(path.parent().unwrap_or(&path)))),"A process is using this project folder. Close terminals and build tools before cleanup.");
        fs::remove_dir_all(&path)?;
        Ok(candidate.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    #[test]
    fn cache_scan_does_not_treat_every_target_as_owned() {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("target")).unwrap();
        let m = Manager::new(
            data.path().to_owned(),
            PathBuf::from("relay"),
            std::sync::Arc::new(|_| {}),
        )
        .unwrap();
        assert!(m.scan_caches(root.path()).unwrap().is_empty());
        assert!(m
            .remove_cache(root.path(), &root.path().join("target"))
            .is_err());
    }
}
