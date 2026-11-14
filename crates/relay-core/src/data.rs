use crate::{manager::Manager, model::App, store::safe_child};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MARKER: &str = ".relay-data-owner";
fn data_path(path: &Path) -> Result<PathBuf> {
    anyhow::ensure!(
        path.is_absolute() && path.is_dir(),
        "Choose an existing app-specific data folder."
    );
    let dirs = directories::BaseDirs::new().context("Your user data folders are unavailable.")?;
    let roots = vec![
        dirs.config_dir().to_owned(),
        dirs.data_dir().to_owned(),
        dirs.data_local_dir().to_owned(),
        dirs.cache_dir().to_owned(),
    ];
    #[cfg(target_os = "macos")]
    let roots = {
        let mut values = roots;
        values.push(dirs.home_dir().join("Library/Preferences"));
        values
    };
    let mut roots = roots;
    roots.sort();
    roots.dedup();
    let canonical = path.canonicalize()?;
    for root in roots.into_iter().filter(|p| p.exists()) {
        let root = root.canonicalize()?;
        if let Ok(relative) = canonical.strip_prefix(&root) {
            if relative.as_os_str().is_empty() {
                continue;
            }
            let first = relative
                .components()
                .next()
                .context("Choose an app-specific subfolder.")?
                .as_os_str()
                .to_string_lossy()
                .to_lowercase();
            anyhow::ensure!(!["microsoft","apple","google","npm","pip","cargo","rustup","relay","code","programs","temp","cache","caches","packages"].contains(&first.as_str()),"This is a shared platform folder. Choose a folder used exclusively by this application.");
            safe_child(
                &root,
                relative
                    .to_str()
                    .context("The folder name has an unsupported encoding.")?,
            )?;
            return Ok(canonical);
        }
    }
    anyhow::bail!("Only app-specific folders inside your user Application Support, AppData, Preferences, or cache folders can be registered. Home folders, source projects, and system folders cannot be removed as app data.")
}
impl Manager {
    pub fn register_data_folder(&self, id: &str, path: &Path) -> Result<()> {
        let _guard = self.lock()?;
        let mut store = self.store()?;
        let mut app = store.app(id)?;
        let path = data_path(path)?;
        anyhow::ensure!(
            !path.starts_with(&self.root),
            "Relay's own data cannot be registered as application data."
        );
        anyhow::ensure!(
            !app.data_folders.iter().any(|p| Path::new(p) != path
                && (path.starts_with(p) || Path::new(p).starts_with(&path))),
            "A registered data folder already contains this path, or is inside it."
        );
        for other in store.apps()?.iter().filter(|a| a.id != id) {
            anyhow::ensure!(
                !other
                    .data_folders
                    .iter()
                    .any(|p| path.starts_with(p) || Path::new(p).starts_with(&path)),
                "This folder overlaps another application's registered data."
            );
        }
        let marker = safe_child(&path, MARKER)?;
        if marker.exists() {
            anyhow::ensure!(
                fs::read_to_string(&marker)? == id,
                "This folder is registered to another application."
            );
        } else {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)?;
            file.write_all(id.as_bytes())?;
            file.sync_all()?;
        }
        let value = path.to_string_lossy().into_owned();
        if !app.data_folders.contains(&value) {
            app.data_folders.push(value);
        }
        store.save(
            &app,
            "Data folder registered",
            "Explicitly registered an app-specific user data folder for optional removal.",
        )
    }
    pub fn forget_data_folder(&self, id: &str, path: &Path) -> Result<()> {
        let _guard = self.lock()?;
        let mut store = self.store()?;
        let mut app = store.app(id)?;
        anyhow::ensure!(
            app.data_folders.iter().any(|p| Path::new(p) == path),
            "This data folder is not registered to the app."
        );
        if path.exists() {
            let path = data_path(path)?;
            let marker = safe_child(&path, MARKER)?;
            anyhow::ensure!(
                fs::read_to_string(&marker)? == id,
                "The data ownership marker changed. Files have been left alone."
            );
            fs::remove_file(marker)?;
        }
        app.data_folders.retain(|p| Path::new(p) != path);
        store.save(
            &app,
            "Data folder disconnected",
            "User files were kept. Relay will no longer remove this folder.",
        )
    }
    pub(crate) fn owned_data_folders(&self, app: &App) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();
        for path in &app.data_folders {
            let path = Path::new(path);
            if !path.exists() {
                continue;
            }
            let path = data_path(path)?;
            anyhow::ensure!(
                !path.starts_with(&self.root),
                "Relay cannot remove its own data as application data."
            );
            anyhow::ensure!(
                fs::read_to_string(safe_child(&path, MARKER)?)? == app.id,
                "The app data ownership marker is missing or changed. No app data was removed."
            );
            result.push(path);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn broad_and_project_directories_cannot_be_owned_app_data() {
        let folder = tempfile::tempdir().unwrap();
        assert!(data_path(folder.path()).is_err());
        let dirs = directories::BaseDirs::new().unwrap();
        assert!(data_path(dirs.home_dir()).is_err());
        assert!(data_path(dirs.config_dir()).is_err());
    }
}
