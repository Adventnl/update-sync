use crate::{
    archive,
    manager::{Manager, Prepared},
    model::*,
    platform,
    store::safe_child,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuildPlan {
    pub stack: String,
    pub steps: Vec<String>,
    pub warning: String,
}

pub fn plan(stacks: &[String]) -> Result<BuildPlan> {
    let has = |name: &str| stacks.iter().any(|s| s == name);
    let (stack, steps) = if has("Tauri") {
        (
            "Tauri",
            vec!["npm ci", "npm exec tauri build -- --no-bundle"],
        )
    } else if has("Electron") {
        (
            "Electron",
            vec!["npm ci", "npm exec electron-builder -- --dir"],
        )
    } else if has("Rust") {
        ("Rust", vec!["cargo build --release"])
    } else if has("C++") {
        (
            "C++",
            vec![
                "cmake -S . -B build -DCMAKE_BUILD_TYPE=Release",
                "cmake --build build --config Release",
                "cmake --install build --config Release --prefix package",
            ],
        )
    } else if has("Java") {
        ("Java", vec!["Gradle or Maven package, with tests skipped"])
    } else if has("Python") {
        (
            "Python",
            vec![
                "Create a private Python environment",
                "Install project requirements and PyInstaller",
                "Package main.py with PyInstaller",
            ],
        )
    } else {
        anyhow::bail!("No supported local desktop build was detected. Publish a portable release package or an Actions artifact instead.")
    };
    Ok(BuildPlan {stack:stack.into(),steps:steps.into_iter().map(Into::into).collect(),warning:"Building executes repository code with your user permissions and network access. The disposable workspace is not a security sandbox. Build only code you trust. Dependency caches and build output are removed afterwards.".into()})
}

impl Manager {
    pub fn build_plan(&self, id: &str) -> Result<BuildPlan> {
        plan(&self.store()?.app(id)?.stacks)
    }
    pub fn local_build(&self, id: &str, trust: bool) -> Result<Prepared> {
        anyhow::ensure!(
            trust,
            "Approve the local build plan before running repository code."
        );
        let _guard = self.lock()?;
        let store = self.store()?;
        let app = store.app(id)?;
        let plan = plan(&app.stacks)?;
        anyhow::ensure!(
            !platform::running(&store.app_dir(id)?),
            "Close the app before preparing a local update."
        );
        let inspection = self.inspect(&app.repository.url(), &app.channel)?;
        anyhow::ensure!(
            inspection.repository_id == app.repository_id,
            "The repository identity changed."
        );
        let commit = inspection
            .head_commit
            .context("Could not resolve the repository's default branch commit.")?;
        anyhow::ensure!(
            commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()),
            "GitHub returned an invalid commit."
        );
        let workspace = tempfile::Builder::new()
            .prefix("build-")
            .tempdir_in(self.root.join("builds"))?;
        fs::write(workspace.path().join(".relay-owned"), "build")?;
        self.report(id, "building", "Downloading a pinned source snapshot");
        let package = Package {
            id: format!("local-{commit}"),
            name: "source.zip".into(),
            url: format!(
                "https://api.github.com/repos/{}/zipball/{commit}",
                app.repository.slug()
            ),
            version: format!("local-{}", &commit[..7]),
            commit: Some(commit),
            digest: None,
            size: 0,
            source: "local".into(),
            published_at: now(),
            compatible: true,
        };
        let source = workspace.path().join("source.zip");
        let github = crate::github::Github::new(store.settings()?.use_github_cli)?;
        let digest = github.download(&package, &source, |_, _| self.check_cancelled())?;
        let extracted = workspace.path().join("source");
        fs::create_dir(&extracted)?;
        archive::extract_zip(&source, &extracted)?;
        let directories: Vec<_> = fs::read_dir(&extracted)?.collect::<std::io::Result<Vec<_>>>()?;
        anyhow::ensure!(
            directories.len() == 1 && directories[0].file_type()?.is_dir(),
            "The source archive has an unexpected layout."
        );
        let project = directories[0].path();
        let output = workspace.path().join("package");
        fs::create_dir(&output)?;
        let build_env = workspace.path().join("environment");
        fs::create_dir(&build_env)?;
        match plan.stack.as_str() {
            "Rust" => {
                let mut args = vec!["build", "--release"];
                if project.join("Cargo.lock").exists() {
                    args.push("--locked");
                }
                self.run_build(id, &project, &build_env, "cargo", &args)?;
                collect_binaries(&project.join("target/release"), &output, &app.id)?;
            }
            "Tauri" | "Electron" => {
                anyhow::ensure!(project.join("package-lock.json").is_file(),"Local Node builds require package-lock.json for npm ci. Publish a remote package or commit the lockfile.");
                self.run_build(id, &project, &build_env, "npm", &["ci"])?;
                if plan.stack == "Tauri" {
                    if cfg!(target_os = "macos") {
                        self.run_build(
                            id,
                            &project,
                            &build_env,
                            "npm",
                            &["exec", "--no", "tauri", "build", "--", "--bundles", "app"],
                        )?;
                        collect_mac_apps(
                            &project.join("src-tauri/target/release/bundle/macos"),
                            &output,
                        )?;
                    } else {
                        self.run_build(
                            id,
                            &project,
                            &build_env,
                            "npm",
                            &["exec", "--no", "tauri", "build", "--", "--no-bundle"],
                        )?;
                        collect_binaries(
                            &project.join("src-tauri/target/release"),
                            &output,
                            &app.id,
                        )?;
                    }
                } else {
                    self.run_build(
                        id,
                        &project,
                        &build_env,
                        "npm",
                        &["exec", "--no", "electron-builder", "--", "--dir"],
                    )?;
                    let dist = project.join("dist");
                    if cfg!(windows) {
                        copy_tree(&dist.join("win-unpacked"), &output)?;
                    } else {
                        collect_mac_apps(&dist, &output)?;
                    }
                }
            }
            "C++" => {
                self.run_build(
                    id,
                    &project,
                    &build_env,
                    "cmake",
                    &["-S", ".", "-B", "build", "-DCMAKE_BUILD_TYPE=Release"],
                )?;
                self.run_build(
                    id,
                    &project,
                    &build_env,
                    "cmake",
                    &["--build", "build", "--config", "Release"],
                )?;
                let destination = output
                    .to_str()
                    .context("Build folder has an invalid encoding.")?;
                self.run_build(
                    id,
                    &project,
                    &build_env,
                    "cmake",
                    &[
                        "--install",
                        "build",
                        "--config",
                        "Release",
                        "--prefix",
                        destination,
                    ],
                )?;
            }
            "Java" => {
                if project.join("pom.xml").exists() {
                    self.run_build(
                        id,
                        &project,
                        &build_env,
                        "mvn",
                        &["-B", "package", "-DskipTests"],
                    )?;
                    collect_jars(&project.join("target"), &output)?;
                } else {
                    self.run_build(
                        id,
                        &project,
                        &build_env,
                        "gradle",
                        &["build", "-x", "test", "--no-daemon"],
                    )?;
                    collect_jars(&project.join("build/libs"), &output)?;
                }
            }
            "Python" => {
                anyhow::ensure!(project.join("main.py").is_file(),"Automatic Python builds currently need main.py. Publish a PyInstaller package for other entry points.");
                let python = if cfg!(windows) { "python" } else { "python3" };
                self.run_build(id, &project, &build_env, python, &["-m", "venv", ".venv"])?;
                let python = project.join(if cfg!(windows) {
                    ".venv/Scripts/python.exe"
                } else {
                    ".venv/bin/python"
                });
                let python = python.to_str().context("Python path is invalid.")?;
                if project.join("requirements.txt").exists() {
                    self.run_build(
                        id,
                        &project,
                        &build_env,
                        python,
                        &["-m", "pip", "install", "-r", "requirements.txt"],
                    )?;
                }
                if project.join("pyproject.toml").exists() || project.join("setup.py").exists() {
                    self.run_build(
                        id,
                        &project,
                        &build_env,
                        python,
                        &["-m", "pip", "install", "."],
                    )?;
                }
                self.run_build(
                    id,
                    &project,
                    &build_env,
                    python,
                    &["-m", "pip", "install", "pyinstaller"],
                )?;
                self.run_build(
                    id,
                    &project,
                    &build_env,
                    python,
                    &[
                        "-m",
                        "PyInstaller",
                        "--noconfirm",
                        "--windowed",
                        "--name",
                        "Application",
                        "main.py",
                    ],
                )?;
                if cfg!(target_os = "macos") {
                    collect_mac_apps(&project.join("dist"), &output)?;
                } else {
                    copy_tree(&project.join("dist"), &output)?;
                }
            }
            _ => anyhow::bail!("This build system is not supported."),
        }
        self.check_cancelled()?;
        let entries = archive::entries(&output)?;
        anyhow::ensure!(!entries.is_empty(),"The build finished without a runnable desktop package. Use GitHub Actions to publish a portable package including its resources.");
        #[cfg(target_os = "macos")]
        for entry in &entries {
            if entry.ends_with(".app") {
                let status = platform::command("/usr/bin/codesign")
                    .args(["--force", "--deep", "--sign", "-", "--timestamp=none"])
                    .arg(output.join(entry))
                    .status()?;
                anyhow::ensure!(status.success(),"The local app could not be ad-hoc signed. Use the project's signed remote package instead.");
            }
        }
        self.persist_prepared(&app, package, digest, &output, entries)
    }
    fn run_build(
        &self,
        id: &str,
        project: &Path,
        environment: &Path,
        program: &str,
        args: &[&str],
    ) -> Result<()> {
        self.check_cancelled()?;
        self.report(
            id,
            "building",
            &format!("Running {program} {}", args.join(" ")),
        );
        let mut cmd = if cfg!(windows) && matches!(program, "npm" | "mvn" | "gradle") {
            let mut c = platform::command("cmd.exe");
            c.args(["/d", "/c", program]);
            c
        } else {
            platform::command(program)
        };
        // Repository code must be trusted even with a clean environment. It can access the user account.
        cmd.env_clear();
        for name in [
            "PATH",
            "SystemRoot",
            "WINDIR",
            "USERPROFILE",
            "HOME",
            "APPDATA",
            "LOCALAPPDATA",
            "TEMP",
            "TMP",
            "NUMBER_OF_PROCESSORS",
            "PROCESSOR_ARCHITECTURE",
        ] {
            if let Some(v) = std::env::var_os(name) {
                cmd.env(name, v);
            }
        }
        cmd.env("CARGO_HOME", environment.join("cargo"))
            .env("npm_config_cache", environment.join("npm"))
            .env("PIP_CACHE_DIR", environment.join("pip"))
            .env("GRADLE_USER_HOME", environment.join("gradle"))
            .env(
                "MAVEN_OPTS",
                format!("-Dmaven.repo.local={}", environment.join("maven").display()),
            )
            .env("CI", "1")
            .args(args)
            .current_dir(project);
        let log = tempfile::tempfile()?;
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd.stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log.try_clone()?));
        let mut child = cmd.spawn().with_context(|| {
            format!("{program} is not available. Install this toolchain or use a GitHub build.")
        })?;
        let started = Instant::now();
        loop {
            if self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
                || started.elapsed() > Duration::from_secs(1800)
                || log.metadata()?.len() > 32 * 1024 * 1024
            {
                #[cfg(windows)]
                {
                    let _ = platform::command("taskkill.exe")
                        .args(["/PID", &child.id().to_string(), "/T", "/F"])
                        .status();
                }
                #[cfg(unix)]
                {
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                }
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("Build stopped. Your installed application has not changed.");
            }
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    use std::io::{Read, Seek, SeekFrom};
                    let mut log = log;
                    let length = log.metadata()?.len();
                    log.seek(SeekFrom::Start(length.saturating_sub(4000)))?;
                    let mut tail = String::new();
                    log.read_to_string(&mut tail)?;
                    anyhow::bail!("{program} failed. Build output:\n{tail}");
                }
                break;
            }
            thread::sleep(Duration::from_millis(150));
        }
        Ok(())
    }
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    anyhow::ensure!(
        source.is_dir(),
        "The expected package folder was not produced."
    );
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        if entry.depth() == 0 {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(source)?
            .to_str()
            .context("Unsupported build output name.")?;
        let target = safe_child(destination, relative)?;
        anyhow::ensure!(
            !crate::store::is_link(&fs::symlink_metadata(entry.path())?),
            "The build output contains links. Publish a signed macOS archive instead."
        );
        if entry.file_type().is_dir() {
            fs::create_dir_all(target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
fn collect_binaries(source: &Path, output: &Path, identity: &str) -> Result<()> {
    #[cfg(not(target_os = "macos"))]
    let _ = identity;
    for entry in
        fs::read_dir(source).context("The build did not produce the expected release folder.")?
    {
        let entry = entry?;
        let path = entry.path();
        #[cfg(target_os = "macos")]
        if path.is_file() && path.extension().is_none() {
            use std::io::Read;
            let mut magic = [0; 4];
            if fs::File::open(&path)?.read_exact(&mut magic).is_ok()
                && [
                    [0xcf, 0xfa, 0xed, 0xfe],
                    [0xfe, 0xed, 0xfa, 0xcf],
                    [0xca, 0xfe, 0xba, 0xbe],
                ]
                .contains(&magic)
            {
                let name = entry
                    .file_name()
                    .to_string_lossy()
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect::<String>();
                anyhow::ensure!(
                    !name.is_empty(),
                    "The native build has no valid application name."
                );
                let bundle = output.join(format!("{name}.app"));
                let content = bundle.join("Contents");
                fs::create_dir_all(content.join("MacOS"))?;
                fs::copy(&path, content.join("MacOS/Application"))?;
                fs::write(content.join("Info.plist"),format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>Application</string><key>CFBundleName</key><string>{name}</string><key>CFBundleIdentifier</key><string>dev.relay.local.{identity}.{name}</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleVersion</key><string>1</string><key>NSHighResolutionCapable</key><true/></dict></plist>"))?;
            }
        }
        if path.is_file()
            && path
                .extension()
                .is_some_and(|e| e == "exe" || e == "dll" || e == "dylib")
        {
            fs::copy(&path, output.join(entry.file_name()))?;
        }
    }
    Ok(())
}
fn collect_mac_apps(source: &Path, output: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let mut walk = walkdir::WalkDir::new(source)
            .follow_links(false)
            .max_depth(5)
            .into_iter();
        let mut count = 0;
        while let Some(entry) = walk.next() {
            let entry = entry?;
            if entry.file_type().is_dir() && entry.path().extension().is_some_and(|e| e == "app") {
                let status = platform::command("/usr/bin/ditto")
                    .arg(entry.path())
                    .arg(output.join(entry.file_name()))
                    .status()?;
                anyhow::ensure!(
                    status.success(),
                    "The built macOS bundle could not be copied."
                );
                walk.skip_current_dir();
                count += 1;
            }
        }
        anyhow::ensure!(count > 0, "The build did not produce a macOS app bundle.");
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (source, output);
        anyhow::bail!("macOS app packaging is only available on macOS.")
    }
}
fn collect_jars(source: &Path, output: &Path) -> Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path: PathBuf = entry.path();
        if path.extension().is_some_and(|e| e == "jar")
            && !entry.file_name().to_string_lossy().contains("sources")
        {
            fs::copy(path, output.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_stack_takes_precedence_over_language() {
        assert_eq!(
            plan(&["Rust".into(), "Tauri".into()]).unwrap().stack,
            "Tauri"
        );
        assert!(plan(&["Swift".into()]).is_err());
    }
}
