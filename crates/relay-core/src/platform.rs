use crate::{
    model::App,
    store::{safe_child, Store},
};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub trait Integration: Send + Sync {
    fn verify(&self, entry: &Path, source: Option<&str>) -> Result<String>;
    fn register(&self, store: &Store, app: &App, launcher: &Path) -> Result<()>;
    fn unregister(&self, store: &Store, app: &App, launcher: &Path) -> Result<()>;
}
pub struct NativeIntegration;
impl Integration for NativeIntegration {
    fn verify(&self, entry: &Path, source: Option<&str>) -> Result<String> {
        let signature = verify_with_policy(entry, source.is_some())?;
        #[cfg(windows)]
        if let Some(source) = source {
            crate::windows_trust::preserve_origin(entry, source)?;
        }
        #[cfg(not(windows))]
        let _ = source;
        Ok(signature)
    }
    fn register(&self, store: &Store, app: &App, launcher: &Path) -> Result<()> {
        integrate(store, app, launcher)
    }
    fn unregister(&self, store: &Store, app: &App, launcher: &Path) -> Result<()> {
        remove_integration(store, app, launcher)
    }
}

pub fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

pub fn default_root() -> Result<PathBuf> {
    Ok(directories::ProjectDirs::from("dev", "Relay", "Relay")
        .context("Your application data folder is unavailable.")?
        .data_local_dir()
        .to_owned())
}

pub fn running(root: &Path) -> bool {
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system
        .processes()
        .values()
        .any(|p| p.exe().is_some_and(|p| p.starts_with(root)))
}

pub fn launch(store: &Store, app: &App) -> Result<()> {
    let version = app.active().context("Install this app first.")?;
    let root = store.version_dir(&app.id, &version.id)?;
    let entry = safe_child(&root, &version.entry)?;
    anyhow::ensure!(
        entry.exists(),
        "The application is missing. Choose Repair to restore it."
    );
    if entry.is_file() {
        let owned = version
            .files
            .iter()
            .find(|f| f.path == version.entry)
            .context("The launch file is not in the ownership receipt.")?;
        anyhow::ensure!(
            crate::archive::hash_file(&entry)? == owned.sha256,
            "The launch file changed since installation. Choose Repair before opening it."
        );
    }
    if entry.extension().is_some_and(|e| e == "jar") {
        validate_jar(&entry)?;
        command("java")
            .arg("-jar")
            .arg(&entry)
            .current_dir(&root)
            .spawn()
            .context("Java is required to open this application.")?;
    } else {
        #[cfg(target_os = "macos")]
        {
            command("/usr/bin/open").arg(&entry).spawn()?;
        }
        #[cfg(windows)]
        {
            crate::windows_trust::open(&entry)?;
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            command(&entry).current_dir(entry.parent().unwrap_or(&root)).spawn().context("The application could not be opened. Check that the package matches your computer.")?;
        }
    }
    Ok(())
}

pub fn open_path(path: &Path) -> Result<()> {
    anyhow::ensure!(path.exists(), "This folder does not exist yet.");
    #[cfg(windows)]
    {
        command("explorer.exe").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        command("/usr/bin/open").arg(path).spawn()?;
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        command("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}
pub fn open_repository(app: &App) -> Result<()> {
    let url = app.repository.url();
    #[cfg(windows)]
    {
        let mut cmd = powershell();
        cmd.env("RELAY_URL", url)
            .arg("Start-Process -FilePath $env:RELAY_URL");
        ensure_success(cmd, "Could not open your browser.")?;
    }
    #[cfg(target_os = "macos")]
    {
        command("/usr/bin/open").arg(url).spawn()?;
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        command("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

fn ensure_success(mut cmd: Command, message: &str) -> Result<()> {
    let result = cmd.output().with_context(|| message.to_owned())?;
    anyhow::ensure!(
        result.status.success(),
        "{} {}",
        message,
        String::from_utf8_lossy(&result.stderr)
            .chars()
            .take(600)
            .collect::<String>()
    );
    Ok(())
}

pub fn verify_signature(entry: &Path) -> Result<String> {
    verify_with_policy(entry, true)
}
fn verify_with_policy(entry: &Path, downloaded: bool) -> Result<String> {
    if entry.extension().is_some_and(|e| e == "jar") {
        validate_jar(entry)?;
        return Ok("Java archive; download hash recorded".into());
    }
    #[cfg(windows)]
    {
        let mut cmd = powershell();
        cmd.env("RELAY_ENTRY", entry).arg("$s=Get-AuthenticodeSignature -LiteralPath $env:RELAY_ENTRY; if ($s.Status -eq 'Valid') { Write-Output 'Signed' } elseif ($s.Status -eq 'NotSigned') { Write-Output 'Unsigned' } else { throw ('Signature validation failed: ' + $s.Status) }");
        if entry.extension().is_some_and(|e| e == "exe") {
            let out = cmd.output()?;
            anyhow::ensure!(
                out.status.success(),
                "The application's code signature is invalid."
            );
            return Ok(String::from_utf8_lossy(&out.stdout).trim().into());
        }
    }
    #[cfg(target_os = "macos")]
    {
        ensure_success(
            {
                let mut c = command("/usr/bin/codesign");
                c.args(["--verify", "--deep", "--strict"]).arg(entry);
                c
            },
            "The app's code signature is missing or invalid.",
        )?;
        if downloaded {
            ensure_success({ let mut c=command("/usr/sbin/spctl"); c.args(["--assess", "--type", "execute"]).arg(entry); c }, "Gatekeeper did not approve this application. Sign and notarize the app before installing it.")?;
            Ok("Signed and approved by Gatekeeper".into())
        } else {
            Ok("Trusted local build; code signature verified".into())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (entry, downloaded);
        Ok("No platform signature".into())
    }
}

fn validate_jar(entry: &Path) -> Result<()> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(fs::File::open(entry)?)
        .context("The Java package is not a valid archive.")?;
    let mut manifest = String::new();
    archive
        .by_name("META-INF/MANIFEST.MF")
        .context("The Java archive has no runnable manifest.")?
        .take(1024 * 1024)
        .read_to_string(&mut manifest)?;
    anyhow::ensure!(
        manifest.lines().any(|line| line.starts_with("Main-Class:")),
        "The Java archive has no Main-Class. Choose a runnable application JAR."
    );
    Ok(())
}

#[cfg(windows)]
fn powershell() -> Command {
    let system = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"));
    let mut cmd = command(system.join("System32/WindowsPowerShell/v1.0/powershell.exe"));
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
    ]);
    cmd
}

pub fn integrate(store: &Store, app: &App, launcher: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let app_dir = store.app_dir(&app.id)?;
        let programs = directories::BaseDirs::new()
            .context("Your user folders are unavailable.")?
            .config_dir()
            .join("Microsoft/Windows/Start Menu/Programs/Relay");
        fs::create_dir_all(&programs)?;
        shortcut(
            &programs.join(format!("{} ({}).lnk", app.name, app.id)),
            app,
            launcher,
            store,
            false,
        )?;
        if app.desktop_shortcut {
            let desktop = directories::UserDirs::new()
                .and_then(|d| d.desktop_dir().map(Path::to_path_buf))
                .context("Desktop folder is unavailable.")?;
            shortcut(
                &desktop.join(format!("{} (Relay {}).lnk", app.name, app.id)),
                app,
                launcher,
                store,
                false,
            )?;
        }
        let key_path = format!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Relay-{}",
            app.id
        );
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(&key_path) {
            let location: String = key.get_value("InstallLocation").unwrap_or_default();
            anyhow::ensure!(
                Path::new(&location) == app_dir,
                "Windows registration belongs to another installation."
            );
        }
        let (key, _) = hkcu.create_subkey(key_path)?;
        key.set_value("DisplayName", &app.name)?;
        key.set_value("Publisher", &format!("{} via Relay", app.repository.owner))?;
        key.set_value("InstallLocation", &app_dir.to_string_lossy().as_ref())?;
        key.set_value(
            "UninstallString",
            &format!(
                "\"{}\" --library \"{}\" --uninstall {}",
                launcher.display(),
                store.root.display(),
                app.id
            ),
        )?;
        key.set_value("NoModify", &1u32)?;
        key.set_value("NoRepair", &1u32)?;
        if let Some(v) = app.active() {
            key.set_value("DisplayVersion", &v.label)?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::symlink;
        let home = directories::BaseDirs::new()
            .context("Home folder unavailable.")?
            .home_dir()
            .to_owned();
        let applications = home.join("Applications/Relay");
        fs::create_dir_all(&applications)?;
        if let Some(v) = app.active() {
            let target = store.version_dir(&app.id, &v.id)?.join(&v.entry);
            let app_root = store.app_dir(&app.id)?;
            if target.extension().is_some_and(|e| e == "app") {
                let link = applications.join(format!(
                    "{} ({}).app",
                    app.name.replace(['/', ':'], "-"),
                    app.id
                ));
                if fs::symlink_metadata(&link).is_ok() {
                    anyhow::ensure!(
                        fs::read_link(&link).is_ok_and(|p| p.starts_with(&app_root)),
                        "An Applications item is owned by another app."
                    );
                }
                let pending = applications.join(format!(".{}.pending", app.id));
                if pending.exists() || fs::symlink_metadata(&pending).is_ok() {
                    anyhow::bail!("A shortcut repair is already pending. Remove the stale .pending link after checking its target.");
                }
                symlink(&target, &pending)?;
                fs::rename(&pending, &link)?;
                let _ = launcher;
            }
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (store, app, launcher);
    }
    Ok(())
}

#[cfg(windows)]
fn shortcut(path: &Path, app: &App, launcher: &Path, store: &Store, remove: bool) -> Result<()> {
    let mut cmd = powershell();
    cmd.env("RELAY_LINK", path)
        .env("RELAY_LAUNCHER", launcher)
        .env("RELAY_ID", &app.id)
        .env("RELAY_NAME", &app.name)
        .env(
            "RELAY_ARGS",
            format!("--library \"{}\" --launch {}", store.root.display(), app.id),
        )
        .env("RELAY_REMOVE", if remove { "1" } else { "0" });
    if let Some(v) = app.active() {
        cmd.env(
            "RELAY_ICON",
            store.version_dir(&app.id, &v.id)?.join(&v.entry),
        );
    }
    cmd.arg(include_str!("scripts/shortcut.ps1"));
    ensure_success(
        cmd,
        "Windows shortcut integration could not be updated. Use Repair shortcuts and retry.",
    )
}

pub fn remove_integration(store: &Store, app: &App, launcher: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let programs = directories::BaseDirs::new()
            .context("User folders unavailable.")?
            .config_dir()
            .join("Microsoft/Windows/Start Menu/Programs/Relay");
        shortcut(
            &programs.join(format!("{} ({}).lnk", app.name, app.id)),
            app,
            launcher,
            store,
            true,
        )?;
        if let Some(desktop) =
            directories::UserDirs::new().and_then(|d| d.desktop_dir().map(Path::to_path_buf))
        {
            shortcut(
                &desktop.join(format!("{} (Relay {}).lnk", app.name, app.id)),
                app,
                launcher,
                store,
                true,
            )?;
        }
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key_path = format!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Relay-{}",
            app.id
        );
        if let Ok(key) = hkcu.open_subkey(&key_path) {
            let location: String = key.get_value("InstallLocation")?;
            anyhow::ensure!(
                Path::new(&location) == store.app_dir(&app.id)?,
                "Windows registration belongs to another installation."
            );
            hkcu.delete_subkey_all(key_path)?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        let home = directories::BaseDirs::new()
            .context("Home folder unavailable.")?
            .home_dir()
            .to_owned();
        let link = home.join("Applications/Relay").join(format!(
            "{} ({}).app",
            app.name.replace(['/', ':'], "-"),
            app.id
        ));
        if fs::symlink_metadata(&link).is_ok() {
            anyhow::ensure!(
                fs::read_link(&link)?.starts_with(store.app_dir(&app.id)?),
                "The Applications item is not owned by Relay."
            );
            fs::remove_file(link)?;
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (store, app, launcher);
    }
    Ok(())
}

pub fn extract_dmg(package: &Path, destination: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let mount = tempfile::tempdir()?;
        ensure_success(
            {
                let mut c = command("/usr/bin/hdiutil");
                c.args([
                    "attach",
                    "-readonly",
                    "-nobrowse",
                    "-noautoopen",
                    "-mountpoint",
                ])
                .arg(mount.path())
                .arg(package);
                c
            },
            "Could not open this disk image.",
        )?;
        let result = (|| {
            let apps: Vec<_> = fs::read_dir(mount.path())?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|s| s == "app"))
                .collect();
            anyhow::ensure!(
                apps.len() == 1,
                "The disk image must contain exactly one application."
            );
            ensure_success(
                {
                    let mut c = command("/usr/bin/ditto");
                    c.arg(apps[0].path())
                        .arg(destination.join(apps[0].file_name()));
                    c
                },
                "Could not copy the application.",
            )
        })();
        let detach = ensure_success(
            {
                let mut c = command("/usr/bin/hdiutil");
                c.arg("detach").arg(mount.path());
                c
            },
            "The disk image could not be ejected.",
        );
        result?;
        detach?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (package, destination);
        anyhow::bail!("Disk images can only be installed on macOS.");
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn shortcut_ownership_normalizes_windows_path_spellings() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(&temp.path().join("library")).unwrap();
        let path = temp.path().join("test.lnk");
        let launcher = std::env::current_exe().unwrap();
        let forward = PathBuf::from(launcher.to_string_lossy().replace('\\', "/"));
        let app = crate::lifecycle_tests::app();
        shortcut(&path, &app, &forward, &store, false).unwrap();
        assert!(path.exists());
        shortcut(&path, &app, &launcher, &store, true).unwrap();
        assert!(!path.exists());
    }
    #[test]
    fn a_shortcut_owned_by_another_launcher_is_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(&temp.path().join("library")).unwrap();
        let path = temp.path().join("test.lnk");
        let launcher = std::env::current_exe().unwrap();
        let app = crate::lifecycle_tests::app();
        shortcut(&path, &app, &launcher, &store, false).unwrap();
        assert!(shortcut(
            &path,
            &app,
            Path::new("C:/Windows/System32/notepad.exe"),
            &store,
            true
        )
        .is_err());
        assert!(path.exists());
    }
}
