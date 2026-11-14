use crate::{
    archive,
    github::Github,
    model::*,
    platform,
    store::{safe_child, Store},
};
use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Manager {
    pub root: PathBuf,
    pub launcher: PathBuf,
    reporter: Reporter,
    pub cancelled: Arc<AtomicBool>,
    pub(crate) integration: Arc<dyn platform::Integration>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prepared {
    pub ticket: String,
    pub app_id: String,
    pub package: Package,
    pub digest: String,
    pub entries: Vec<String>,
    pub previous: Option<String>,
}

impl Manager {
    pub fn new(root: PathBuf, launcher: PathBuf, reporter: Reporter) -> Result<Self> {
        let store = Store::open(&root)?;
        Ok(Self {
            root: store.root,
            launcher,
            reporter,
            cancelled: Arc::new(AtomicBool::new(false)),
            integration: Arc::new(platform::NativeIntegration),
        })
    }
    pub fn store(&self) -> Result<Store> {
        Store::open(&self.root)
    }
    pub fn lock(&self) -> Result<fs::File> {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(safe_child(&self.root, "operation.lock")?)?;
        file.try_lock_exclusive()
            .context("Another Relay operation is in progress. Wait for it to finish.")?;
        self.cancelled.store(false, Ordering::Relaxed);
        Ok(file)
    }
    pub fn check_cancelled(&self) -> Result<()> {
        anyhow::ensure!(
            !self.cancelled.load(Ordering::Relaxed),
            "Operation cancelled. Your installed version has not changed."
        );
        Ok(())
    }
    pub fn operation_active(&self) -> bool {
        let Ok(path) = safe_child(&self.root, "operation.lock") else {
            return true;
        };
        let Ok(file) = fs::OpenOptions::new().read(true).write(true).open(path) else {
            return false;
        };
        file.try_lock_exclusive().is_err()
    }
    pub fn report(&self, id: &str, stage: &str, message: &str) {
        (self.reporter)(Progress {
            app_id: id.into(),
            stage: stage.into(),
            message: message.into(),
            completed: 0,
            total: None,
        });
    }
    fn github(&self) -> Result<Github> {
        Github::new(self.store()?.settings()?.use_github_cli)
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let store = self.store()?;
        Ok(Snapshot {
            apps: store.apps()?,
            activity: store.activity()?,
            settings: store.settings()?,
            root: self.root.to_string_lossy().into(),
            platform: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            auth: self
                .github()
                .map(|g| g.auth_label())
                .unwrap_or_else(|e| format!("Sign-in needs attention: {e}")),
        })
    }
    pub fn inspect(&self, url: &str, channel: &Channel) -> Result<Inspection> {
        self.github()?.inspect(&Repository::parse(url)?, channel)
    }
    pub fn workflows(&self, id: &str) -> Result<Vec<Workflow>> {
        self.github()?.workflows(&self.store()?.app(id)?.repository)
    }
    pub fn request_build(&self, id: &str, workflow: u64) -> Result<()> {
        let _guard = self.lock()?;
        let mut store = self.store()?;
        let app = store.app(id)?;
        self.github()?.dispatch(&app.repository, workflow)?;
        store.save(&app,"Build requested","Requested a GitHub Actions workflow on the default branch. Check for updates when it finishes.")
    }
    pub fn add(&self, url: &str, channel: Channel) -> Result<App> {
        let _guard = self.lock()?;
        let inspected = self.inspect(url, &channel)?;
        let mut store = self.store()?;
        let id = format!("gh-{}", inspected.repository_id);
        if let Ok(mut app) = store.app(&id) {
            app.tracked = true;
            app.channel = channel;
            app.available = select_update(&inspected.packages, app.active());
            app.repository = inspected.repository;
            store.save(
                &app,
                "Tracking resumed",
                "Reconnected to GitHub. Existing files were left in place.",
            )?;
            return Ok(app);
        }
        let app = App {
            id,
            repository_id: inspected.repository_id,
            name: inspected.repository.name.clone(),
            repository: inspected.repository,
            description: inspected.description,
            stacks: inspected.stacks,
            channel,
            tracked: true,
            active_version: None,
            versions: vec![],
            available: inspected.packages.into_iter().find(|p| p.compatible),
            checked_at: Some(now()),
            check_error: None,
            desktop_shortcut: false,
            data_folders: vec![],
        };
        store.save(&app, "Added", "Added to your library.")?;
        Ok(app)
    }
    pub fn check_updates(&self) -> Result<()> {
        {
            let _guard = self.lock()?;
        }
        let mut store = self.store()?;
        let github = Github::new(store.settings()?.use_github_cli)?;
        for original in store.apps()?.into_iter().filter(|a| a.tracked) {
            self.check_cancelled()?;
            self.report(&original.id, "checking", "Checking GitHub");
            let inspected = github.inspect(&original.repository, &original.channel);
            self.check_cancelled()?;
            let _guard = self.lock()?;
            let mut app = store.app(&original.id)?;
            if !app.tracked || app.channel != original.channel {
                continue;
            }
            match inspected {
                Ok(info) => {
                    anyhow::ensure!(
                        info.repository_id == app.repository_id,
                        "The repository identity changed. Add the repository again."
                    );
                    app.available = select_update(&info.packages, app.active());
                    app.repository = info.repository;
                    app.check_error = None;
                }
                Err(e) => {
                    app.check_error = Some(e.to_string());
                }
            }
            app.checked_at = Some(now());
            store.save(&app, "", "")?;
        }
        self.report("", "done", "Update check complete");
        Ok(())
    }
    pub fn prepare(&self, id: &str, package_id: &str, allow_unverified: bool) -> Result<Prepared> {
        let _guard = self.lock()?;
        let store = self.store()?;
        let app = store.app(id)?;
        anyhow::ensure!(
            !platform::running(&store.app_dir(id)?),
            "Close {} before installing an update. Relay will not interrupt it.",
            app.name
        );
        self.report(id, "resolving", "Finding your package");
        let github = Github::new(store.settings()?.use_github_cli)?;
        let inspection = github.inspect(&app.repository, &app.channel)?;
        anyhow::ensure!(
            inspection.repository_id == app.repository_id,
            "The GitHub repository identity no longer matches this app."
        );
        let package = inspection
            .packages
            .into_iter()
            .find(|p| p.id == package_id)
            .context("This package is no longer available. Check for updates and try again.")?;
        anyhow::ensure!(
            package.compatible,
            "This package targets another operating system or processor."
        );
        anyhow::ensure!(
            package.digest.is_some() || allow_unverified,
            "This package has no publisher checksum. Review and approve it before installing."
        );
        let temp = tempfile::Builder::new()
            .prefix("download-")
            .tempdir_in(self.root.join("staging"))?;
        fs::write(temp.path().join(".relay-owned"), "download")?;
        let download = temp.path().join("package");
        let last = std::sync::Mutex::new(Instant::now() - Duration::from_secs(1));
        let digest = github.download(&package, &download, |completed, total| {
            self.check_cancelled()?;
            let mut last = last
                .lock()
                .map_err(|_| anyhow::anyhow!("Progress state unavailable."))?;
            if last.elapsed() > Duration::from_millis(100) || total == Some(completed) {
                (self.reporter)(Progress {
                    app_id: id.into(),
                    stage: "downloading".into(),
                    message: "Downloading from GitHub".into(),
                    completed,
                    total,
                });
                *last = Instant::now();
            }
            Ok(())
        })?;
        self.check_cancelled()?;
        self.report(id, "verifying", "Verifying and unpacking");
        let content = temp.path().join("content");
        fs::create_dir(&content)?;
        let candidates = archive::unpack_application(&download, &package.name, &content)?;
        self.check_cancelled()?;
        anyhow::ensure!(!candidates.is_empty(),"No runnable app was found. Choose a portable desktop package, rather than source code, test output, or an installer.");
        self.persist_prepared(&app, package, digest, &content, candidates)
    }
    pub(crate) fn persist_prepared(
        &self,
        app: &App,
        package: Package,
        digest: String,
        content: &Path,
        entries: Vec<String>,
    ) -> Result<Prepared> {
        let ticket = uuid::Uuid::new_v4().to_string();
        let destination = safe_child(&self.root, &format!("staging/{ticket}"))?;
        fs::create_dir(&destination)?;
        let plan = Prepared {
            ticket,
            app_id: app.id.clone(),
            package,
            digest,
            entries,
            previous: app.active_version.clone(),
        };
        // A complete plan precedes promotion. Interrupted staging is never an active version.
        write_durable(&destination.join("plan.json"), &serde_json::to_vec(&plan)?)?;
        write_durable(&destination.join(".relay-owned"), plan.ticket.as_bytes())?;
        fs::rename(content, destination.join("content"))?;
        self.report(&app.id, "ready", "Ready to install");
        Ok(plan)
    }
    pub fn commit(&self, ticket: &str, entry: &str) -> Result<App> {
        let _guard = self.lock()?;
        uuid::Uuid::parse_str(ticket)?;
        let staged = safe_child(&self.root, &format!("staging/{ticket}"))?;
        anyhow::ensure!(
            fs::read_to_string(staged.join(".relay-owned"))? == ticket,
            "The staged install has no ownership receipt."
        );
        let plan: Prepared = serde_json::from_slice(&fs::read(staged.join("plan.json"))?)?;
        anyhow::ensure!(
            plan.ticket == ticket && plan.entries.iter().any(|e| e == entry),
            "Choose a launch file from the package."
        );
        let mut store = self.store()?;
        let mut app = store.app(&plan.app_id)?;
        anyhow::ensure!(
            app.active_version == plan.previous,
            "This app changed while the package was being prepared. Prepare it again."
        );
        anyhow::ensure!(
            !platform::running(&store.app_dir(&app.id)?),
            "Close {} before updating. Your download is ready and can be retried.",
            app.name
        );
        let content = safe_child(&staged, "content")?;
        let launch = safe_child(&content, entry)?;
        self.report(&app.id, "verifying", "Checking application integrity");
        let source = if plan.package.source == "local" {
            None
        } else {
            Some(plan.package.url.as_str())
        };
        let signature = self.integration.verify(&launch, source)?;
        let files = archive::inventory(&content)?;
        let size = files.iter().map(|f| f.size).sum();
        self.check_cancelled()?;
        let version = InstalledVersion {
            id: uuid::Uuid::new_v4().to_string(),
            label: plan.package.version.clone(),
            commit: plan.package.commit.clone(),
            installed_at: now(),
            entry: entry.into(),
            package_digest: plan.digest,
            source: plan.package.id,
            files,
            size,
        };
        let app_dir = store.app_dir(&app.id)?;
        fs::create_dir_all(app_dir.join("versions"))?;
        let destination = store.version_dir(&app.id, &version.id)?;
        fs::write(
            content.join(".relay-owned"),
            format!("{}:{}", app.id, version.id),
        )?;
        write_durable(
            &staged.join("promotion.json"),
            &serde_json::to_vec(&version)?,
        )?;
        sync_tree(&content)?;
        fs::rename(&content, &destination)?;
        let old = app.clone();
        app.active_version = Some(version.id.clone());
        app.versions.insert(0, version);
        app.available = None;
        app.check_error = None;
        self.report(&app.id, "installing", "Registering your application");
        // Shortcuts on Windows resolve the database pointer, so they survive every version.
        // On macOS the Applications symlink is swapped with an atomic rename.
        if let Err(error) = self.integration.register(&store, &app, &self.launcher) {
            if old.active().is_some() {
                self.integration
                    .register(&store, &old, &self.launcher)
                    .context("Integration failed and the old shortcut needs repair.")?;
            } else {
                self.integration.unregister(&store, &app, &self.launcher)?;
            }
            fs::rename(&destination, &content)?;
            fs::remove_file(staged.join("promotion.json"))?;
            return Err(error
                .context("The previous active version is unchanged. Repair shortcuts and retry."));
        }
        let action = if old.active().is_some() {
            "Updated"
        } else {
            "Installed"
        };
        if let Err(error) = store.save(
            &app,
            action,
            &format!("{} · {}", plan.package.version, signature),
        ) {
            if old.active().is_some() {
                self.integration.register(&store, &old, &self.launcher)?;
            } else {
                self.integration.unregister(&store, &app, &self.launcher)?;
            }
            fs::rename(&destination, &content)?;
            fs::remove_file(staged.join("promotion.json"))?;
            return Err(error.context(
                "Could not commit the installation. Your previous version is still active.",
            ));
        }
        fs::remove_dir_all(&staged)
            .context("Installed successfully, but the staging folder needs storage cleanup.")?;
        self.report(&app.id, "done", "Ready to open");
        Ok(app)
    }
    pub fn discard(&self, ticket: &str) -> Result<()> {
        let _guard = self.lock()?;
        uuid::Uuid::parse_str(ticket)?;
        let path = safe_child(&self.root, &format!("staging/{ticket}"))?;
        anyhow::ensure!(
            fs::read_to_string(path.join(".relay-owned"))? == ticket,
            "This folder is not a prepared install."
        );
        fs::remove_dir_all(path)?;
        Ok(())
    }
    pub fn prepared_installs(&self) -> Result<Vec<Prepared>> {
        let mut result = Vec::new();
        for entry in fs::read_dir(self.root.join("staging"))? {
            let entry = entry?;
            let ticket = entry.file_name().to_string_lossy().into_owned();
            if uuid::Uuid::parse_str(&ticket).is_err() {
                continue;
            }
            let path = safe_child(&self.root, &format!("staging/{ticket}"))?;
            if !path.join("content").is_dir() || path.join("promotion.json").exists() {
                continue;
            }
            if fs::read_to_string(path.join(".relay-owned"))? != ticket {
                continue;
            }
            let plan: Prepared = serde_json::from_slice(&fs::read(path.join("plan.json"))?)?;
            anyhow::ensure!(
                plan.ticket == ticket,
                "A prepared download has an invalid receipt."
            );
            result.push(plan);
        }
        Ok(result)
    }
    pub fn launch(&self, id: &str) -> Result<()> {
        let store = self.store()?;
        let guard = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(safe_child(&self.root, "operation.lock")?)?;
        FileExt::try_lock_shared(&guard)
            .context("This app is being changed. Open it again when the operation finishes.")?;
        platform::launch(&store, &store.app(id)?)
    }
    pub fn recover(&self) -> Result<()> {
        let _guard = self.lock()?;
        let store = self.store()?;
        for entry in fs::read_dir(self.root.join("staging"))? {
            let entry = entry?;
            let ticket = entry.file_name().to_string_lossy().into_owned();
            if uuid::Uuid::parse_str(&ticket).is_err() {
                continue;
            }
            let staged = safe_child(&self.root, &format!("staging/{ticket}"))?;
            if !staged.join("promotion.json").is_file() {
                continue;
            }
            anyhow::ensure!(
                fs::read_to_string(staged.join(".relay-owned"))? == ticket,
                "An interrupted install has no valid ownership marker."
            );
            let plan: Prepared = serde_json::from_slice(&fs::read(staged.join("plan.json"))?)?;
            let version: InstalledVersion =
                serde_json::from_slice(&fs::read(staged.join("promotion.json"))?)?;
            let app = store.app(&plan.app_id)?;
            if app.active_version.as_deref() == Some(&version.id) {
                self.integration.register(&store, &app, &self.launcher)?;
                fs::remove_dir_all(&staged)?;
            } else {
                if app.active().is_some() {
                    self.integration.register(&store, &app, &self.launcher)?;
                } else {
                    self.integration.unregister(&store, &app, &self.launcher)?;
                }
                let promoted = self.assert_owned_version(&store, &app.id, &version.id)?;
                if promoted.exists() && !staged.join("content").exists() {
                    fs::rename(promoted, staged.join("content"))?;
                }
                fs::remove_file(staged.join("promotion.json"))?;
            }
        }
        Ok(())
    }
    pub fn track(&self, id: &str, tracked: bool) -> Result<()> {
        let _guard = self.lock()?;
        let mut store = self.store()?;
        let mut app = store.app(id)?;
        app.tracked = tracked;
        store.save(
            &app,
            if tracked {
                "Tracking resumed"
            } else {
                "Tracking stopped"
            },
            "Application files and shortcuts were left in place.",
        )
    }
    pub fn channel(&self, id: &str, channel: Channel) -> Result<()> {
        let _guard = self.lock()?;
        let mut store = self.store()?;
        let mut app = store.app(id)?;
        app.channel = channel;
        app.available = None;
        store.save(
            &app,
            "Channel changed",
            "Check for updates to find packages on this channel.",
        )
    }
    pub fn shortcuts(&self, id: &str, desktop: bool) -> Result<()> {
        let _guard = self.lock()?;
        let mut store = self.store()?;
        let mut app = store.app(id)?;
        anyhow::ensure!(app.active().is_some(), "Install the app first.");
        app.desktop_shortcut = desktop;
        self.integration.register(&store, &app, &self.launcher)?;
        store.save(
            &app,
            "Shortcuts repaired",
            "Application launch integration is up to date.",
        )
    }
    pub fn verify(&self, id: &str) -> Result<Vec<String>> {
        let store = self.store()?;
        let app = store.app(id)?;
        let active = app.active().context("Install the app first.")?;
        let root = store.version_dir(id, &active.id)?;
        let current = archive::inventory(&root)?;
        let map: std::collections::HashMap<_, _> =
            current.into_iter().map(|f| (f.path, f.sha256)).collect();
        Ok(active
            .files
            .iter()
            .filter(|f| map.get(&f.path) != Some(&f.sha256))
            .map(|f| f.path.clone())
            .collect())
    }
    pub fn rollback(&self, id: &str, version_id: &str) -> Result<()> {
        let _guard = self.lock()?;
        let mut store = self.store()?;
        let mut app = store.app(id)?;
        anyhow::ensure!(
            !platform::running(&store.app_dir(id)?),
            "Close the app before switching versions."
        );
        let version = app
            .versions
            .iter()
            .find(|v| v.id == version_id)
            .context("That version is no longer installed.")?;
        let root = store.version_dir(id, version_id)?;
        let current = archive::inventory(&root)?;
        let hashes: std::collections::HashMap<_, _> =
            current.into_iter().map(|f| (f.path, f.sha256)).collect();
        anyhow::ensure!(
            version
                .files
                .iter()
                .all(|f| hashes.get(&f.path) == Some(&f.sha256)),
            "This version's files changed. Repair it before switching."
        );
        let old = app.clone();
        app.active_version = Some(version_id.into());
        self.integration.register(&store, &app, &self.launcher)?;
        if let Err(e) = store.save(
            &app,
            "Version restored",
            &format!(
                "Switched to {}.",
                app.active().context("Missing active version.")?.label
            ),
        ) {
            self.integration.register(&store, &old, &self.launcher)?;
            return Err(e);
        }
        Ok(())
    }
    pub fn uninstall(&self, id: &str, remove_data: bool) -> Result<()> {
        let _guard = self.lock()?;
        let mut store = self.store()?;
        let mut app = store.app(id)?;
        anyhow::ensure!(
            !platform::running(&store.app_dir(id)?),
            "Close {} before removing it. Relay will not stop it for you.",
            app.name
        );
        // Verify all receipts before changing anything. External user folders are never inferred.
        for version in &app.versions {
            self.assert_owned_version(&store, id, &version.id)?;
        }
        let owned_data = if remove_data {
            self.owned_data_folders(&app)?
        } else {
            vec![]
        };
        self.integration.unregister(&store, &app, &self.launcher)?;
        // Clear the active pointer before deleting. Interrupted deletion can be retried safely.
        app.active_version = None;
        app.available = None;
        store.save(
            &app,
            "Uninstall started",
            "Removing store-owned application versions.",
        )?;
        let versions = app.versions.clone();
        for version in versions {
            let path = store.version_dir(id, &version.id)?;
            if path.exists() {
                fs::remove_dir_all(path)?;
            }
            app.versions.retain(|v| v.id != version.id);
            store.save(&app, "", "")?;
        }
        if remove_data {
            for path in owned_data {
                fs::remove_dir_all(path)?;
            }
            app.data_folders.clear();
            let data = safe_child(&store.app_dir(id)?, "data")?;
            if data.exists() {
                fs::remove_dir_all(data)?;
            }
        }
        store.save(&app,"Uninstalled",if remove_data{"Removed installed versions and registered app data. Unregistered folders were left alone."}else{"Removed installed versions. User data was kept."})?;
        self.report(id, "done", "Application removed");
        Ok(())
    }
    pub(crate) fn assert_owned_version(
        &self,
        store: &Store,
        id: &str,
        version: &str,
    ) -> Result<PathBuf> {
        let path = store.version_dir(id, version)?;
        if path.exists() {
            anyhow::ensure!(
                fs::read_to_string(path.join(".relay-owned"))? == format!("{id}:{version}"),
                "Ownership cannot be verified. No files were removed."
            );
        }
        Ok(path)
    }
    pub fn reveal(&self, id: Option<&str>, data: bool) -> Result<()> {
        let store = self.store()?;
        let path = if let Some(id) = id {
            let path = store.app_dir(id)?;
            if data {
                let path = safe_child(&path, "data")?;
                fs::create_dir_all(&path)?;
                path
            } else {
                path
            }
        } else {
            self.root.clone()
        };
        platform::open_path(&path)
    }
    pub fn open_repository(&self, id: &str) -> Result<()> {
        let store = self.store()?;
        platform::open_repository(&store.app(id)?)
    }
}

fn write_durable(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn sync_tree(root: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .contents_first(true)
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(entry.path())?
                .sync_all()?;
        }
        #[cfg(unix)]
        if entry.file_type().is_dir() {
            fs::File::open(entry.path())?.sync_all()?;
        }
    }
    Ok(())
}

pub(crate) fn select_update(
    packages: &[Package],
    installed: Option<&InstalledVersion>,
) -> Option<Package> {
    let newest = packages.iter().find(|p| p.compatible)?;
    let group: Vec<_> = packages
        .iter()
        .filter(|p| {
            p.compatible && p.version == newest.version && p.published_at == newest.published_at
        })
        .collect();
    // A release can publish GNU/MSVC or several app variants. Installing one must
    // not make another asset from that same release appear as a new version.
    if installed.is_some_and(|v| group.iter().any(|p| p.id == v.source)) {
        return None;
    }
    group
        .into_iter()
        .enumerate()
        .max_by_key(|(index, p)| {
            let mut score = 0;
            if cfg!(windows) && p.name.contains("msvc") {
                score += 10;
            }
            if p.name.ends_with(".zip") {
                score += 3;
            }
            if let Some(v) = installed {
                for flavor in ["msvc", "gnu", "universal", "arm64", "aarch64", "x86_64"] {
                    if v.entry.contains(flavor) && p.name.contains(flavor) {
                        score += 20;
                    }
                }
            }
            (score, usize::MAX - index)
        })
        .map(|(_, p)| p.clone())
}
