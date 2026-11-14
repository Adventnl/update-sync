use crate::{
    archive,
    manager::{Manager, Prepared},
    model::*,
    platform,
    recipes::{self, BuildConfig, BuildSystem},
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

const BUILD_LIMIT: u64 = 12 * 1024 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuildPlan {
    pub stack: String,
    pub steps: Vec<String>,
    pub warning: String,
}

pub fn plan(stacks: &[String]) -> Result<BuildPlan> {
    let stack = [
        "Tauri", "Electron", "Rust", "Cargo", "C++", "Cmake", "Java", "Maven", "Gradle", "Python",
        "Go", ".NET", "Dotnet",
    ]
    .into_iter()
    .find(|name| stacks.iter().any(|s| s == name))
    .context("Build configuration needed: select a build system and project folder.")?;
    Ok(BuildPlan { stack: stack.into(), steps: vec!["Download the latest main commit".into(), "Detect and compile the application".into(), "Install and remove build files".into()], warning: "Builds execute repository code with your user permissions. Managed folders are not a security sandbox.".into() })
}

impl Manager {
    pub fn build_plan(&self, id: &str) -> Result<BuildPlan> {
        plan(&self.store()?.app(id)?.stacks)
    }
    pub fn local_build(&self, id: &str, trust: bool) -> Result<Prepared> {
        anyhow::ensure!(trust, "Approve running repository build code.");
        self.build_source(id, None)
    }
    pub fn build_source(&self, id: &str, configuration: Option<BuildConfig>) -> Result<Prepared> {
        let _guard = self.lock()?;
        self.clean_interrupted_builds()?;
        let mut store = self.store()?;
        let mut app = store.app(id)?;
        anyhow::ensure!(
            !platform::running(&store.app_dir(id)?),
            "Close the app before updating. Your installed version has not changed."
        );
        self.report(id, "resolving", "Resolving the latest main commit");
        let inspection = self.inspect(&app.repository.url(), &Channel::Main)?;
        anyhow::ensure!(
            inspection.repository_id == app.repository_id,
            "The repository identity changed."
        );
        let commit = inspection.head_commit.context("Could not resolve main.")?;
        let mut package = crate::github::source_package(&inspection.repository, &commit)?;
        // Source builds bypass downloaded-package trust handling at activation.
        package.source = "local".into();
        let workspace = tempfile::Builder::new()
            .prefix("build-")
            .tempdir_in(safe_child(&self.root, "builds")?)?;
        // Build tools frequently concatenate paths with '/'. Windows verbatim paths
        // reject that spelling, so give toolchains ordinary absolute paths.
        let workspace_root = dunce::simplified(workspace.path()).to_path_buf();
        fs::write(workspace_root.join(".relay-owned"), "build")?;
        let result = (|| {
            self.report(
                id,
                "downloading",
                "Downloading source at the pinned main commit",
            );
            let source = workspace_root.join("source.zip");
            let github = crate::github::Github::new(store.settings()?.use_github_cli)?;
            let digest = github.download(&package, &source, |_, _| self.check_cancelled())?;
            let extracted = workspace_root.join("source");
            fs::create_dir(&extracted)?;
            archive::extract_zip(&source, &extracted)?;
            fs::remove_file(source)?;
            let directories: Vec<_> =
                fs::read_dir(&extracted)?.collect::<std::io::Result<Vec<_>>>()?;
            anyhow::ensure!(
                directories.len() == 1 && directories[0].file_type()?.is_dir(),
                "The source archive has an unexpected layout."
            );
            let root = directories[0].path();
            anyhow::ensure!(!root.join(".gitmodules").exists(), "This project uses Git submodules. Source snapshots do not include their contents; this repository needs a self-contained source layout.");
            let config = configuration
                .as_ref()
                .or(app.build_config.as_ref())
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| recipes::detect(&root))?;
            if configuration.is_some() {
                config.validate(&root)?;
                // "Save and retry" must retain the user's choice even if compilation fails.
                app.build_config = Some(config.clone());
                store.save(&app, "", "")?;
            }
            let output = workspace_root.join("package");
            fs::create_dir(&output)?;
            let environment = workspace_root.join("environment");
            fs::create_dir(&environment)?;
            self.compile_project(id, &root, &environment, &output, &app.id, &config)?;
            anyhow::ensure!(managed_size(&workspace_root) <= BUILD_LIMIT, "Build stopped because it exceeded the 12 GB workspace limit. Your installed application has not changed.");
            for resource in &config.resources {
                let project = config.validate(&root)?;
                let source = safe_child(&project, resource)?;
                let destination = safe_child(&output, resource)?;
                if source.is_dir() {
                    fs::create_dir_all(&destination)?;
                    copy_tree(&source, &destination)?;
                } else {
                    fs::create_dir_all(destination.parent().context("Resource has no parent.")?)?;
                    fs::copy(source, destination)?;
                }
            }
            self.check_cancelled()?;
            anyhow::ensure!(managed_size(&workspace_root) <= BUILD_LIMIT, "Build stopped because its output exceeded the 12 GB workspace limit. Your installed application has not changed.");
            let entries = archive::entries(&output)?;
            anyhow::ensure!(!entries.is_empty(), "Build configuration needed: the build produced no runnable application. Select the output folder and application entry.");
            let entries = choose_entries(
                entries,
                config.entry.as_deref(),
                app.active().map(|v| v.entry.as_str()),
                &app.repository.name,
            )?;
            #[cfg(target_os = "macos")]
            for entry in &entries {
                if entry.ends_with(".app") {
                    let status = platform::command("/usr/bin/codesign")
                        .args(["--force", "--deep", "--sign", "-", "--timestamp=none"])
                        .arg(output.join(entry))
                        .status()?;
                    anyhow::ensure!(
                        status.success(),
                        "The built app could not be ad-hoc signed."
                    );
                }
            }
            let prepared = self.persist_prepared(&app, package, digest, &output, entries)?;
            app.channel = Channel::Main;
            app.stacks = vec![format!("{:?}", config.system)];
            store.save(&app, "", "")?;
            Ok(prepared)
        })();
        // TempDir is also dropped on every error. Startup removes owned crash leftovers.
        let cleanup = workspace.close();
        match result {
            Ok(prepared) => {
                cleanup.context("Build finished, but temporary files could not be removed. Retry cleanup in Storage.")?;
                Ok(prepared)
            }
            Err(e) => Err(e),
        }
    }

    pub(crate) fn compile_project(
        &self,
        id: &str,
        root: &Path,
        environment: &Path,
        output: &Path,
        identity: &str,
        config: &BuildConfig,
    ) -> Result<()> {
        let root = dunce::simplified(root);
        let environment = dunce::simplified(environment);
        let output = dunce::simplified(output);
        let project = config.validate(root)?;
        let run =
            |program: &str, args: &[&str]| self.run_build(id, &project, environment, program, args);
        let target_dir = environment.join("target");
        match config.system {
            BuildSystem::Cargo => {
                let mut args = vec!["build", "--release"];
                if project.join("Cargo.lock").exists() || root.join("Cargo.lock").exists() {
                    args.push("--locked");
                }
                if let Some(target) = &config.target {
                    args.extend(["--bin", target]);
                }
                run("cargo", &args)?;
                if config.output.is_none() {
                    collect_binaries(&target_dir.join("release"), output, identity)?;
                }
            }
            BuildSystem::Tauri | BuildSystem::Electron => {
                let (manager, install): (&str, Vec<&str>) = if project
                    .join("pnpm-lock.yaml")
                    .is_file()
                {
                    ("pnpm", vec!["install", "--frozen-lockfile"])
                } else if project.join("yarn.lock").is_file() {
                    let modern = project.join(".yarnrc.yml").exists();
                    (
                        "yarn",
                        vec![
                            "install",
                            if modern {
                                "--immutable"
                            } else {
                                "--frozen-lockfile"
                            },
                        ],
                    )
                } else if project.join("package-lock.json").is_file() {
                    ("npm", vec!["ci"])
                } else {
                    anyhow::bail!("Node projects need a committed npm, pnpm, or Yarn lockfile. Add a lockfile to the repository, then retry.");
                };
                run(manager, &install)?;
                // Invoke the installed project CLI directly. Never download an unpinned CLI with npx.
                let cli =
                    project
                        .join("node_modules")
                        .join(if config.system == BuildSystem::Tauri {
                            "@tauri-apps/cli/tauri.js"
                        } else {
                            "electron-builder/cli.js"
                        });
                anyhow::ensure!(cli.is_file(), "The project must declare its desktop build CLI as a dependency. Add it to the repository, then retry.");
                let cli = cli.to_str().context("Invalid CLI path.")?;
                if config.system == BuildSystem::Tauri {
                    if cfg!(target_os = "macos") {
                        run("node", &[cli, "build", "--bundles", "app"])?;
                        if config.output.is_none() {
                            collect_mac_apps(&target_dir.join("release/bundle/macos"), output)?;
                        }
                    } else {
                        run("node", &[cli, "build", "--no-bundle"])?;
                        if config.output.is_none() {
                            collect_binaries(&target_dir.join("release"), output, identity)?;
                            collect_tauri_resources(&project, output)?;
                        }
                    }
                } else {
                    let manifest: serde_json::Value =
                        serde_json::from_slice(&fs::read(project.join("package.json"))?)?;
                    if manifest
                        .get("scripts")
                        .and_then(|s| s.get("build"))
                        .is_some()
                    {
                        run(manager, &["run", "build"])?;
                    }
                    run(
                        "node",
                        &[cli, "--dir", "--config.directories.output=.relay-electron"],
                    )?;
                    if config.output.is_none() {
                        let dist = project.join(".relay-electron");
                        if cfg!(windows) {
                            copy_tree(
                                &dist.join(if cfg!(target_arch = "aarch64") {
                                    "win-arm64-unpacked"
                                } else {
                                    "win-unpacked"
                                }),
                                output,
                            )?;
                        } else {
                            collect_mac_apps(&dist, output)?;
                        }
                    }
                }
            }
            BuildSystem::Cmake => {
                run(
                    "cmake",
                    &[
                        "-S",
                        ".",
                        "-B",
                        ".relay-build",
                        "-DCMAKE_BUILD_TYPE=Release",
                    ],
                )?;
                let jobs = build_jobs().to_string();
                run(
                    "cmake",
                    &[
                        "--build",
                        ".relay-build",
                        "--config",
                        "Release",
                        "--parallel",
                        &jobs,
                    ],
                )?;
                if config.output.is_none() {
                    run(
                        "cmake",
                        &[
                            "--install",
                            ".relay-build",
                            "--config",
                            "Release",
                            "--prefix",
                            output.to_str().context("Invalid output folder.")?,
                        ],
                    )?;
                }
            }
            BuildSystem::Maven => {
                let repository =
                    format!("-Dmaven.repo.local={}", environment.join("maven").display());
                let wrapper = if cfg!(windows) { "mvnw.cmd" } else { "./mvnw" };
                run(
                    if project.join(wrapper).is_file() {
                        wrapper
                    } else {
                        "mvn"
                    },
                    &["-B", &repository, "package", "-DskipTests"],
                )?;
                if config.output.is_none() {
                    collect_jars(&project.join("target"), output)?;
                }
            }
            BuildSystem::Gradle => {
                let wrapper = if cfg!(windows) {
                    "gradlew.bat"
                } else {
                    "./gradlew"
                };
                run(
                    if project.join(wrapper).is_file() {
                        wrapper
                    } else {
                        "gradle"
                    },
                    &[
                        "build",
                        "-x",
                        "test",
                        "--no-daemon",
                        "--max-workers",
                        &build_jobs().to_string(),
                    ],
                )?;
                if config.output.is_none() {
                    collect_jars(&project.join("build/libs"), output)?;
                }
            }
            BuildSystem::Python => {
                let entry = config.target.as_deref().unwrap_or("main.py");
                anyhow::ensure!(
                    safe_child(&project, entry)?.is_file(),
                    "Build configuration needed: select the Python entry file."
                );
                let python = if cfg!(windows) { "python" } else { "python3" };
                run(python, &["-m", "venv", ".venv"])?;
                let python = project.join(if cfg!(windows) {
                    ".venv/Scripts/python.exe"
                } else {
                    ".venv/bin/python"
                });
                let python = python.to_str().context("Invalid Python path.")?;
                if project.join("requirements.txt").is_file() {
                    run(python, &["-m", "pip", "install", "-r", "requirements.txt"])?;
                }
                if project.join("pyproject.toml").is_file() || project.join("setup.py").is_file() {
                    run(python, &["-m", "pip", "install", "."])?;
                }
                run(python, &["-m", "pip", "install", "pyinstaller==6.16.0"])?;
                run(
                    python,
                    &[
                        "-m",
                        "PyInstaller",
                        "--noconfirm",
                        "--windowed",
                        "--name",
                        "Application",
                        entry,
                    ],
                )?;
                if config.output.is_none() {
                    if cfg!(target_os = "macos") {
                        collect_mac_apps(&project.join("dist"), output)?;
                    } else {
                        copy_tree(&project.join("dist/Application"), output)?;
                    }
                }
            }
            BuildSystem::Go => {
                let target = config.target.as_deref().unwrap_or(".");
                let target = if target == "." {
                    target.to_owned()
                } else {
                    format!("./{}", target.trim_start_matches("./"))
                };
                let binary = environment.join(if cfg!(windows) {
                    "go-bin/Application.exe"
                } else {
                    "go-bin/Application"
                });
                fs::create_dir_all(binary.parent().context("Missing binary parent.")?)?;
                run(
                    "go",
                    &[
                        "build",
                        "-mod=readonly",
                        "-trimpath",
                        "-p",
                        &build_jobs().to_string(),
                        "-o",
                        binary.to_str().context("Invalid output path.")?,
                        &target,
                    ],
                )?;
                if config.output.is_none() {
                    collect_binaries(
                        binary.parent().context("Missing output folder.")?,
                        output,
                        identity,
                    )?;
                }
            }
            BuildSystem::Dotnet => {
                let candidates = fs::read_dir(&project)?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.extension()
                            .is_some_and(|e| e == "csproj" || e == "fsproj")
                    })
                    .collect::<Vec<_>>();
                let target = if let Some(target) = &config.target {
                    target.clone()
                } else {
                    anyhow::ensure!(
                        candidates.len() == 1,
                        "Build configuration needed: select the .NET application project file."
                    );
                    candidates[0]
                        .file_name()
                        .context("Invalid .NET project.")?
                        .to_string_lossy()
                        .into_owned()
                };
                let rid = format!(
                    "{}-{}",
                    if cfg!(windows) { "win" } else { "osx" },
                    if cfg!(target_arch = "aarch64") {
                        "arm64"
                    } else {
                        "x64"
                    }
                );
                let published = environment.join("publish");
                run(
                    "dotnet",
                    &[
                        "publish",
                        &target,
                        "-c",
                        "Release",
                        "-r",
                        &rid,
                        "--self-contained",
                        "true",
                        "-o",
                        published.to_str().context("Invalid output path.")?,
                    ],
                )?;
                if config.output.is_none() {
                    if cfg!(windows) {
                        copy_tree(&published, output)?;
                    } else {
                        anyhow::bail!("Build configuration needed: select the macOS .app output folder for this .NET project.");
                    }
                }
            }
        }
        if let Some(folder) = &config.output {
            copy_tree(&safe_child(&project, folder)?, output)?;
        }
        Ok(())
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
        let mut cmd = if cfg!(windows)
            && matches!(
                program,
                "npm" | "pnpm" | "yarn" | "mvn" | "gradle" | "mvnw.cmd" | "gradlew.bat"
            ) {
            anyhow::ensure!(
                args.iter()
                    .all(|arg| !arg.contains(['%', '!', '&', '|', '<', '>', '^', '"', '\r', '\n'])),
                "A build argument contains unsupported shell characters."
            );
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
            "RUSTUP_HOME",
            "JAVA_HOME",
            "DOTNET_ROOT",
            "GOROOT",
            "SDKROOT",
            "DEVELOPER_DIR",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
            "VSINSTALLDIR",
            "VCINSTALLDIR",
            "VCToolsInstallDir",
            "WindowsSdkDir",
            "WindowsSDKVersion",
            "INCLUDE",
            "LIB",
            "LIBPATH",
            "VSCMD_ARG_TGT_ARCH",
        ] {
            if let Some(v) = std::env::var_os(name) {
                cmd.env(name, v);
            }
        }
        // Desktop launches do not inherit a Visual Studio developer shell.
        // Discover the installed compiler and SDK without changing the user's environment.
        #[cfg(windows)]
        if let Some(tool) = cc::windows_registry::find_tool(std::env::consts::ARCH, "cl.exe") {
            cmd.envs(tool.get_envs());
        }
        cmd.env("CARGO_HOME", environment.join("cargo"))
            .env("CARGO_TARGET_DIR", environment.join("target"))
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_BUILD_JOBS", build_jobs().to_string())
            .env("npm_config_cache", environment.join("npm"))
            .env("npm_config_store_dir", environment.join("pnpm"))
            .env("YARN_CACHE_FOLDER", environment.join("yarn"))
            .env("YARN_ENABLE_GLOBAL_CACHE", "false")
            .env("PIP_CACHE_DIR", environment.join("pip"))
            .env("PIP_NO_CACHE_DIR", "1")
            .env("PIP_NO_INPUT", "1")
            .env("GRADLE_USER_HOME", environment.join("gradle"))
            .env("MAVEN_USER_HOME", environment.join("maven-user"))
            .env("GOCACHE", environment.join("go-cache"))
            .env("GOMODCACHE", environment.join("go-mod"))
            .env("GOPATH", environment.join("go"))
            .env("NUGET_PACKAGES", environment.join("nuget"))
            .env("DOTNET_CLI_HOME", environment.join("dotnet"))
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env("XDG_CACHE_HOME", environment.join("cache"))
            .env("CCACHE_DIR", environment.join("ccache"))
            .env("SCCACHE_DIR", environment.join("sccache"))
            .env("CI", "1")
            .args(args)
            .current_dir(project);
        let scratch = environment.join("tmp");
        fs::create_dir_all(&scratch)?;
        cmd.env("TEMP", &scratch)
            .env("TMP", &scratch)
            .env("TMPDIR", &scratch);
        cmd.stdin(Stdio::null());
        let log = tempfile::tempfile_in(environment)?;
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd.stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log.try_clone()?));
        let mut child = cmd.spawn().with_context(|| {
            format!("{program} is not available. Install the {program} toolchain, then retry Install. No build commands are needed.")
        })?;
        #[cfg(windows)]
        let _job = match BuildJob::attach(&child) {
            Ok(job) => job,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let started = Instant::now();
        let mut last_size_check = started - Duration::from_secs(3);
        loop {
            let over_budget = if last_size_check.elapsed() >= Duration::from_secs(2) {
                last_size_check = Instant::now();
                managed_size(environment.parent().context("Missing build workspace.")?)
                    > BUILD_LIMIT
            } else {
                false
            };
            if self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
                || started.elapsed() > Duration::from_secs(1800)
                || log.metadata()?.len() > 32 * 1024 * 1024
                || over_budget
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
                anyhow::bail!(
                    "Build stopped{}. Your installed application has not changed.",
                    if over_budget {
                        " because it exceeded the 12 GB workspace limit"
                    } else {
                        " after cancellation or a time/output limit"
                    }
                );
            }
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    use std::io::{Read, Seek, SeekFrom};
                    let mut log = log;
                    let length = log.metadata()?.len();
                    log.seek(SeekFrom::Start(length.saturating_sub(4000)))?;
                    let mut bytes = Vec::new();
                    log.read_to_end(&mut bytes)?;
                    let tail = String::from_utf8_lossy(&bytes);
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
    let mut count = 0;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path: PathBuf = entry.path();
        if path.extension().is_some_and(|e| e == "jar")
            && !entry.file_name().to_string_lossy().contains("sources")
        {
            use std::io::Read;
            let mut jar = zip::ZipArchive::new(fs::File::open(&path)?)?;
            let mut manifest = String::new();
            if let Ok(file) = jar.by_name("META-INF/MANIFEST.MF") {
                file.take(1024 * 1024).read_to_string(&mut manifest)?;
            }
            if !manifest.lines().any(|line| line.starts_with("Main-Class:")) {
                continue;
            }
            fs::copy(path, output.join(entry.file_name()))?;
            count += 1;
        }
    }
    anyhow::ensure!(count > 0, "Build configuration needed: no executable JAR with Main-Class was produced. Select a runnable application output folder.");
    Ok(())
}

fn collect_tauri_resources(project: &Path, output: &Path) -> Result<()> {
    let native = project.join("src-tauri");
    let config_path = native.join("tauri.conf.json");
    anyhow::ensure!(config_path.is_file(), "Build configuration needed: select an application output folder for this Tauri configuration format.");
    let mut config: serde_json::Value = serde_json::from_slice(&fs::read(config_path)?)?;
    // Platform bundle settings replace the same top-level bundle fields.
    let platform_config = native.join("tauri.windows.conf.json");
    if platform_config.is_file() {
        let platform: serde_json::Value = serde_json::from_slice(&fs::read(platform_config)?)?;
        if let Some(bundle) = platform.get("bundle").and_then(|v| v.as_object()) {
            for (key, value) in bundle {
                config["bundle"][key] = value.clone();
            }
        }
    }
    if let Some(resources) = config.pointer("/bundle/resources") {
        match resources {
            serde_json::Value::Array(paths) => {
                for path in paths {
                    let path = path
                        .as_str()
                        .context("Build configuration needed: invalid Tauri resource path.")?;
                    copy_resource_pattern(&native, path, None, output)?;
                }
            }
            serde_json::Value::Object(paths) => {
                for (source, destination) in paths {
                    copy_resource_pattern(
                        &native,
                        source,
                        Some(
                            destination
                                .as_str()
                                .context("Invalid Tauri resource destination.")?,
                        ),
                        output,
                    )?;
                }
            }
            serde_json::Value::Null => {}
            _ => anyhow::bail!("Build configuration needed: unsupported Tauri resource layout."),
        }
    }
    if let Some(binaries) = config
        .pointer("/bundle/externalBin")
        .and_then(|v| v.as_array())
    {
        let triple = format!(
            "{}-pc-windows-msvc",
            if cfg!(target_arch = "aarch64") {
                "aarch64"
            } else {
                "x86_64"
            }
        );
        for binary in binaries {
            let binary = binary.as_str().context("Invalid sidecar path.")?;
            let path = safe_child(&native, &format!("{binary}-{triple}.exe"))?;
            let name = Path::new(binary)
                .file_name()
                .context("Invalid sidecar name.")?
                .to_string_lossy();
            fs::copy(path, safe_child(output, &format!("{name}.exe"))?)
                .context("Build configuration needed: the Tauri sidecar was not produced.")?;
        }
    }
    Ok(())
}

fn copy_resource_pattern(
    root: &Path,
    pattern: &str,
    destination: Option<&str>,
    output: &Path,
) -> Result<()> {
    let source = safe_child(root, pattern)?;
    if source.is_dir() {
        let target = safe_child(output, destination.unwrap_or(pattern).trim_end_matches('/'))?;
        fs::create_dir_all(&target)?;
        return copy_tree(&source, &target);
    }
    let pattern = glob::Pattern::new(pattern)?;
    let mut count = 0;
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(32)
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if !pattern.matches(&relative)
            && !relative.starts_with(&format!("{}/", pattern.as_str().trim_end_matches('/')))
        {
            continue;
        }
        let target = match destination {
            Some(folder) if folder.ends_with('/') || folder.is_empty() => {
                format!("{folder}{}", entry.file_name().to_string_lossy())
            }
            Some(path) => path.to_owned(),
            None => relative.clone(),
        };
        let source = safe_child(root, &relative)?;
        let target = safe_child(output, &target)?;
        anyhow::ensure!(
            !target.exists(),
            "Build configuration needed: resource destinations overlap."
        );
        fs::create_dir_all(target.parent().context("Missing resource parent.")?)?;
        fs::copy(source, target)?;
        count += 1;
    }
    anyhow::ensure!(
        count > 0,
        "Build configuration needed: resource pattern matched no files."
    );
    Ok(())
}

fn build_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().min(4))
        .unwrap_or(2)
}

// Closing the job also terminates compiler descendants after cancellation, failures,
// normal completion, or a Relay crash. No detached cache daemons are retained.
#[cfg(windows)]
struct BuildJob(windows::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl BuildJob {
    fn attach(child: &std::process::Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{Foundation::HANDLE, System::JobObjects::*};
        unsafe {
            let job = Self(CreateJobObjectW(None, None)?);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            )?;
            AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle()))?;
            Ok(job)
        }
    }
}
#[cfg(windows)]
impl Drop for BuildJob {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
fn managed_size(root: &Path) -> u64 {
    walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .fold(0u64, |size, meta| size.saturating_add(meta.len()))
}
fn choose_entries(
    entries: Vec<String>,
    configured: Option<&str>,
    previous: Option<&str>,
    name: &str,
) -> Result<Vec<String>> {
    if let Some(entry) = configured {
        anyhow::ensure!(
            entries.iter().any(|e| e == entry),
            "Build configuration needed: configured launch entry was not produced."
        );
        return Ok(vec![entry.into()]);
    }
    if let Some(entry) = previous.filter(|p| entries.iter().any(|e| e == p)) {
        return Ok(vec![entry.into()]);
    }
    let matches: Vec<_> = entries
        .iter()
        .filter(|e| {
            Path::new(e)
                .file_stem()
                .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(name))
        })
        .cloned()
        .collect();
    if matches.len() == 1 {
        Ok(matches)
    } else {
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_choice_survives_updates_and_missing_explicit_entry_is_an_error() {
        let entries = vec!["Helper.exe".into(), "Desktop.exe".into()];
        assert_eq!(
            choose_entries(entries.clone(), None, Some("Desktop.exe"), "repository").unwrap(),
            vec!["Desktop.exe"]
        );
        assert_eq!(
            choose_entries(entries.clone(), None, None, "Helper").unwrap(),
            vec!["Helper.exe"]
        );
        assert_eq!(
            choose_entries(entries.clone(), None, None, "unknown")
                .unwrap()
                .len(),
            2
        );
        assert!(choose_entries(entries, Some("Missing.exe"), None, "unknown").is_err());
    }
    #[test]
    fn tauri_resource_directories_preserve_structure_and_reject_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let native = tmp.path().join("src-tauri");
        fs::create_dir_all(native.join("assets/nested")).unwrap();
        fs::write(native.join("assets/nested/model.bin"), "model").unwrap();
        fs::write(
            native.join("tauri.conf.json"),
            r#"{"bundle":{"resources":{"assets":"data"}}}"#,
        )
        .unwrap();
        let out = tmp.path().join("output");
        fs::create_dir(&out).unwrap();
        collect_tauri_resources(tmp.path(), &out).unwrap();
        assert!(out.join("data/nested/model.bin").is_file());
        assert!(copy_resource_pattern(&native, "../outside", None, &out).is_err());
    }
    #[test]
    fn non_runnable_jars_are_not_presented_as_applications() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let mut writer =
            zip::ZipWriter::new(fs::File::create(tmp.path().join("library.jar")).unwrap());
        writer
            .start_file(
                "META-INF/MANIFEST.MF",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"Manifest-Version: 1.0\n").unwrap();
        writer.finish().unwrap();
        assert!(collect_jars(tmp.path(), out.path()).is_err());
    }
}
