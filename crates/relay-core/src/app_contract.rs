//! Versioned app builds: language-independent commands, complete output, explicit launch entry.
use crate::{
    recipes::{BuildConfig, BuildSystem},
    store::safe_child,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub const MANIFEST: &str = "relay.app.json";
pub const TARGETS: [&str; 4] = [
    "windows-x86_64",
    "windows-aarch64",
    "macos-x86_64",
    "macos-aarch64",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppContract {
    pub version: u32,
    #[serde(default = "root_project")]
    pub project: String,
    pub targets: BTreeMap<String, AppTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppTarget {
    pub steps: Vec<BuildStep>,
    pub output: String,
    pub entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildStep {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedApp {
    pub platform: String,
    pub project: String,
    #[serde(flatten)]
    pub target: AppTarget,
}

fn root_project() -> String {
    ".".into()
}

// Portable paths have one spelling on every host. In particular, reject Windows
// paths while validating on macOS too, before handing anything to a toolchain.
fn relative_path(value: &str) -> Result<()> {
    anyhow::ensure!(
        !value.is_empty()
            && value.len() <= 500
            && !value.contains(['\\', ':'])
            && !value.chars().any(char::is_control)
            && value
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        "App contract paths must be relative, use /, and stay inside the project."
    );
    Ok(())
}

fn project_path(root: &Path, value: &str) -> Result<PathBuf> {
    if value == "." {
        return Ok(root.to_owned());
    }
    relative_path(value)?;
    safe_child(root, value)
}

pub fn current_target() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

pub fn load(root: &Path) -> Result<SelectedApp> {
    load_for(root, &current_target())
}

pub fn exists(root: &Path) -> Result<bool> {
    match fs::symlink_metadata(root.join(MANIFEST)) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub fn load_for(root: &Path, platform: &str) -> Result<SelectedApp> {
    (|| {
        let path = safe_child(root, MANIFEST)?;
        anyhow::ensure!(
            fs::metadata(&path)?.len() <= 256 * 1024,
            "App contract exceeds 256 KB."
        );
        let contract: AppContract = serde_json::from_slice(&fs::read(path)?)?;
        anyhow::ensure!(
            contract.version == 1,
            "Unsupported app contract version {}. Use version 1.",
            contract.version
        );
        anyhow::ensure!(
            !contract.targets.is_empty(),
            "Declare at least one app target."
        );
        let project = project_path(root, &contract.project)?;
        anyhow::ensure!(project.is_dir(), "The app project folder does not exist.");
        for (name, target) in &contract.targets {
            anyhow::ensure!(
                TARGETS.contains(&name.as_str()),
                "Unknown app target {name}. Expected an operating system and CPU from: {}.",
                TARGETS.join(", ")
            );
            relative_path(&target.output)?;
            relative_path(&target.entry)?;
            safe_child(&project, &target.output)?;
            safe_child(&project.join(&target.output), &target.entry)?;
            let extension = Path::new(&target.entry)
                .extension()
                .and_then(|x| x.to_str());
            anyhow::ensure!(
                extension == Some("jar")
                    || (name.starts_with("windows-") && extension == Some("exe"))
                    || (name.starts_with("macos-") && extension == Some("app")),
                "Target {name} needs an .exe (Windows), .app (macOS), or executable .jar entry."
            );
            anyhow::ensure!(
                !target.steps.is_empty() && target.steps.len() <= 64,
                "Declare between 1 and 64 build steps for {name}."
            );
            for step in &target.steps {
                anyhow::ensure!(
                    !step.program.is_empty()
                        && step.program.len() <= 500
                        && !step.program.starts_with('-')
                        && !step.program.chars().any(char::is_control),
                    "Invalid build program for {name}."
                );
                if step.program.contains('/') {
                    relative_path(&step.program)?;
                    safe_child(&project, &step.program)?;
                } else {
                    anyhow::ensure!(
                        !step.program.contains(['\\', ':', ' '])
                            && step.program != "."
                            && step.program != "..",
                        "Use a tool name or project-relative program path."
                    );
                }
                anyhow::ensure!(
                    step.args.len() <= 256
                        && step
                            .args
                            .iter()
                            .all(|a| a.len() <= 16_384 && !a.contains('\0')),
                    "Too many or invalid build arguments for {name}."
                );
            }
        }
        let target = contract
            .targets
            .get(platform)
            .with_context(|| {
                format!("This app has no build for {platform}. Add that target to {MANIFEST}.")
            })?
            .clone();
        Ok(SelectedApp {
            platform: platform.into(),
            project: contract.project,
            target,
        })
    })()
    .with_context(|| format!("App build contract error in {MANIFEST}"))
}

impl SelectedApp {
    pub fn config(&self) -> BuildConfig {
        BuildConfig {
            system: BuildSystem::App,
            project: self.project.clone(),
            target: None,
            output: Some(self.target.output.clone()),
            entry: Some(self.target.entry.clone()),
            resources: vec![],
        }
    }

    pub fn program(&self, project: &Path, program: &str) -> Result<String> {
        if program.contains('/') {
            Ok(safe_child(project, program)?
                .to_str()
                .context("Invalid build program path.")?
                .to_owned())
        } else {
            Ok(program.into())
        }
    }
}

/// Explicit entries bypass filename guesses (for example, an app named Helper).
/// They still have to use a launch format supported by Relay's platform layer.
pub fn validate_output(output: &Path, entry: &str) -> Result<Vec<String>> {
    relative_path(entry)?;
    let path = safe_child(output, entry)?;
    let valid = match path.extension().and_then(|x| x.to_str()) {
        Some("jar") => path.is_file(),
        Some("exe") if cfg!(windows) => path.is_file(),
        Some("app") if cfg!(target_os = "macos") => {
            path.is_dir()
                && safe_child(&path, "Contents/Info.plist")?.is_file()
                && safe_child(&path, "Contents/MacOS")?.is_dir()
        }
        _ => false,
    };
    anyhow::ensure!(valid, "The declared app entry {entry} was not produced. Check the build steps and output folder in {MANIFEST}.");
    Ok(vec![entry.into()])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write(root: &Path, value: serde_json::Value) {
        fs::write(root.join(MANIFEST), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    fn fixture() -> serde_json::Value {
        serde_json::json!({"version":1,"targets":{
            "windows-x86_64":{"steps":[{"program":"new-language-compiler","args":["--app","app.src"]}],"output":"dist/app","entry":"Helper.exe"},
            "macos-aarch64":{"steps":[{"program":"sh","args":["scripts/build.sh"]}],"output":"dist/mac","entry":"Duct Tape.app"}
        }})
    }
    #[test]
    fn selects_app_without_language_detection() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), fixture());
        let win = load_for(root.path(), "windows-x86_64").unwrap();
        assert_eq!(win.target.steps[0].program, "new-language-compiler");
        assert_eq!(win.config().entry.as_deref(), Some("Helper.exe"));
        assert_eq!(
            load_for(root.path(), "macos-aarch64")
                .unwrap()
                .target
                .output,
            "dist/mac"
        );
        assert!(format!(
            "{:#}",
            load_for(root.path(), "windows-aarch64").unwrap_err()
        )
        .contains("no build for windows-aarch64"));
    }
    #[test]
    fn rejects_unknown_fields_versions_targets_and_path_escapes() {
        let root = tempfile::tempdir().unwrap();
        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("command", serde_json::json!("guess")),
            ("project", serde_json::json!("../outside")),
        ] {
            let mut bad = fixture();
            bad[field] = value;
            write(root.path(), bad);
            assert!(load_for(root.path(), "windows-x86_64").is_err(), "{field}");
        }
        for path in [
            ".",
            "../escape",
            "/tmp/app",
            "C:/app",
            "dist\\app",
            "dist/../app",
            "dist//app",
        ] {
            let mut bad = fixture();
            bad["targets"]["windows-x86_64"]["output"] = path.into();
            write(root.path(), bad);
            assert!(load_for(root.path(), "windows-x86_64").is_err(), "{path}");
        }
        let mut bad = fixture();
        bad["targets"]["unknown"] = bad["targets"]["windows-x86_64"].clone();
        write(root.path(), bad);
        assert!(load_for(root.path(), "windows-x86_64").is_err());
        let mut bad = fixture();
        bad["targets"]["windows-x86_64"]["steps"] = serde_json::json!([]);
        write(root.path(), bad);
        assert!(load_for(root.path(), "windows-x86_64").is_err());
    }
    #[test]
    fn explicit_entry_does_not_use_name_heuristics() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("Helper.jar"), "fixture").unwrap();
        assert!(crate::archive::entries(root.path()).unwrap().is_empty());
        assert_eq!(
            validate_output(root.path(), "Helper.jar").unwrap(),
            vec!["Helper.jar"]
        );
        assert!(validate_output(root.path(), "Missing.jar").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_linked_output_and_manifest() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), root.path().join("dist")).unwrap();
        write(root.path(), fixture());
        assert!(load_for(root.path(), "windows-x86_64").is_err());
        fs::remove_file(root.path().join(MANIFEST)).unwrap();
        fs::write(
            outside.path().join("manifest"),
            serde_json::to_vec(&fixture()).unwrap(),
        )
        .unwrap();
        symlink(outside.path().join("manifest"), root.path().join(MANIFEST)).unwrap();
        assert!(load_for(root.path(), "windows-x86_64").is_err());
        fs::remove_file(outside.path().join("manifest")).unwrap();
        fs::write(root.path().join("Cargo.toml"), "[workspace]").unwrap();
        assert!(exists(root.path()).unwrap());
        assert!(crate::recipes::detect(root.path()).is_err());
    }
}
