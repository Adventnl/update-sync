//! Deterministic source recipes and repository-owned application contracts.
use crate::store::safe_child;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BuildSystem {
    App,
    Tauri,
    Electron,
    Cargo,
    Cmake,
    Maven,
    Gradle,
    Python,
    Go,
    Dotnet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildConfig {
    pub system: BuildSystem,
    #[serde(default = "root_project")]
    pub project: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub resources: Vec<String>,
}
fn root_project() -> String {
    ".".into()
}
impl BuildConfig {
    pub fn new(system: BuildSystem, project: String) -> Self {
        Self {
            system,
            project,
            target: None,
            output: None,
            entry: None,
            resources: vec![],
        }
    }
    pub fn validate(&self, root: &Path) -> Result<PathBuf> {
        let project = if self.project == "." {
            root.to_owned()
        } else {
            safe_child(root, &self.project)?
        };
        anyhow::ensure!(
            project.is_dir(),
            "Build configuration needed: project folder does not exist."
        );
        for value in self
            .target
            .iter()
            .chain(self.output.iter())
            .chain(self.entry.iter())
            .chain(self.resources.iter())
        {
            if self.system != BuildSystem::App {
                anyhow::ensure!(
                    !value.starts_with('-')
                        && value.len() <= 500
                        && !value.contains(['\r', '\n', '%', '!', '&', '|', '<', '>', '^', '"']),
                    "Invalid build configuration value."
                );
            }
            // Targets are names or project-relative paths, never command-line switches.
            if value != "." {
                safe_child(&project, value)?;
            }
        }
        Ok(project)
    }
}

fn systems(path: &Path) -> Result<Vec<BuildSystem>> {
    let mut systems = vec![];
    if path.join("package.json").is_file() {
        let package: serde_json::Value =
            serde_json::from_slice(&fs::read(path.join("package.json"))?)
                .context("Invalid package.json.")?;
        let dependency = |name: &str| {
            ["dependencies", "devDependencies"]
                .iter()
                .any(|key| package.get(key).and_then(|v| v.get(name)).is_some())
        };
        if path.join("src-tauri").is_dir() || dependency("@tauri-apps/cli") {
            systems.push(BuildSystem::Tauri);
        }
        if dependency("electron") {
            systems.push(BuildSystem::Electron);
        }
    }
    if systems.is_empty() {
        for (files, system) in [
            (vec!["Cargo.toml"], BuildSystem::Cargo),
            (vec!["CMakeLists.txt"], BuildSystem::Cmake),
            (vec!["pom.xml"], BuildSystem::Maven),
            (
                vec!["build.gradle", "build.gradle.kts"],
                BuildSystem::Gradle,
            ),
            (
                vec!["main.py", "pyproject.toml", "setup.py", "requirements.txt"],
                BuildSystem::Python,
            ),
            (vec!["go.mod"], BuildSystem::Go),
        ] {
            if files.iter().any(|f| path.join(f).is_file()) {
                systems.push(system);
            }
        }
        if fs::read_dir(path)?.filter_map(|e| e.ok()).any(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x == "csproj" || x == "fsproj")
        }) {
            systems.push(BuildSystem::Dotnet);
        }
    }
    Ok(systems)
}

pub fn detect(root: &Path) -> Result<BuildConfig> {
    if crate::app_contract::exists(root)? {
        return crate::app_contract::load(root).map(|app| app.config());
    }
    // An optional repository-owned recipe uses the same restricted schema as saved configuration.
    let manifest = root.join("relay.build.json");
    if manifest.is_file() {
        let config: BuildConfig = serde_json::from_slice(&fs::read(manifest)?)
            .context("Build configuration needed: invalid relay.build.json.")?;
        config.validate(root)?;
        return Ok(config);
    }
    let root_systems = systems(root)?;
    let mut candidates = vec![];
    let mut walker = walkdir::WalkDir::new(root)
        .min_depth(1)
        .max_depth(4)
        .follow_links(false)
        .into_iter();
    let mut count = 0;
    while let Some(entry) = walker.next() {
        count += 1;
        anyhow::ensure!(
            count <= 20_000,
            "Build configuration needed: choose the application project folder."
        );
        let entry = entry?;
        if !entry.file_type().is_dir() {
            continue;
        }
        if [
            ".git",
            "node_modules",
            "target",
            "vendor",
            ".venv",
            "build",
            "src-tauri",
            "tests",
            "examples",
        ]
        .contains(&entry.file_name().to_string_lossy().as_ref())
        {
            walker.skip_current_dir();
            continue;
        }
        for system in systems(entry.path())? {
            candidates.push(BuildConfig::new(
                system,
                entry
                    .path()
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
            ));
        }
    }
    // A desktop frontend in a Cargo workspace takes precedence over its workspace manifest.
    let desktop: Vec<_> = candidates
        .iter()
        .filter(|c| matches!(c.system, BuildSystem::Tauri | BuildSystem::Electron))
        .collect();
    if root_systems == [BuildSystem::Cargo] && desktop.len() == 1 {
        return Ok(desktop[0].clone());
    }
    if root_systems.len() == 1 {
        return Ok(BuildConfig::new(root_systems[0].clone(), ".".into()));
    }
    if root_systems.is_empty() && candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }
    let choices = candidates
        .iter()
        .map(|c| format!("{} ({:?})", c.project, c.system))
        .collect::<Vec<_>>()
        .join(", ");
    anyhow::bail!("Build configuration needed: {}. Select a build system and project folder once. Detected projects: {}", if root_systems.is_empty() && candidates.is_empty() { "no supported application manifest was found" } else { "more than one build system or application was found" }, if choices.is_empty() { "repository root" } else { &choices })
}

/// Explicit user choices win. A repository app contract supersedes older saved
/// recipe guesses; its commands and output paths are reloaded at every commit.
pub fn resolve(
    root: &Path,
    explicit: Option<&BuildConfig>,
    saved: Option<&BuildConfig>,
) -> Result<BuildConfig> {
    let selected = if let Some(config) = explicit {
        config.clone()
    } else if crate::app_contract::exists(root)? {
        return detect(root);
    } else if let Some(config) = saved {
        config.clone()
    } else {
        return detect(root);
    };
    if selected.system == BuildSystem::App {
        return crate::app_contract::load(root).map(|app| app.config());
    }
    selected.validate(root)?;
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framework_precedes_language_and_reads_real_dependencies() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[workspace]").unwrap();
        fs::create_dir_all(tmp.path().join("apps/desktop/src-tauri")).unwrap();
        fs::write(
            tmp.path().join("apps/desktop/package.json"),
            r#"{"devDependencies":{"@tauri-apps/cli":"2"}}"#,
        )
        .unwrap();
        assert_eq!(
            detect(tmp.path()).unwrap(),
            BuildConfig::new(BuildSystem::Tauri, "apps/desktop".into())
        );
    }
    #[test]
    fn ambiguity_requires_configuration_and_config_rejects_escape() {
        let tmp = tempfile::tempdir().unwrap();
        for name in ["one", "two"] {
            fs::create_dir(tmp.path().join(name)).unwrap();
            fs::write(tmp.path().join(name).join("go.mod"), "module example").unwrap();
        }
        assert!(detect(tmp.path())
            .unwrap_err()
            .to_string()
            .contains("configuration needed"));
        let mut config = BuildConfig::new(BuildSystem::Go, "../escape".into());
        assert!(config.validate(tmp.path()).is_err());
        config.project = ".".into();
        config.target = Some("--help".into());
        assert!(config.validate(tmp.path()).is_err());
    }
    #[test]
    fn committed_recipe_is_reusable_without_commands() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("relay.build.json"),
            r#"{"system":"python","target":"app.py","resources":["assets"]}"#,
        )
        .unwrap();
        assert_eq!(
            detect(tmp.path()).unwrap().target.as_deref(),
            Some("app.py")
        );
        fs::write(
            tmp.path().join("relay.build.json"),
            r#"{"system":"python","command":"arbitrary"}"#,
        )
        .unwrap();
        assert!(detect(tmp.path()).is_err());
    }
    #[test]
    fn app_contract_precedes_saved_language_recipe_and_is_reloaded() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[workspace]").unwrap();
        let legacy = BuildConfig::new(BuildSystem::Cargo, ".".into());
        let manifest = |entry: &str| {
            serde_json::json!({"version":1,"targets":{
                (crate::app_contract::current_target()): {"steps":[{"program":"custom-compiler"}],"output":"dist/app","entry":entry}
            }})
        };
        let entry = if cfg!(target_os = "macos") {
            "Desktop.app"
        } else {
            "Desktop.exe"
        };
        fs::write(
            tmp.path().join(crate::app_contract::MANIFEST),
            manifest(entry).to_string(),
        )
        .unwrap();
        let first = resolve(tmp.path(), None, Some(&legacy)).unwrap();
        assert_eq!(first.system, BuildSystem::App);
        assert_eq!(resolve(tmp.path(), Some(&legacy), None).unwrap(), legacy);
        let entry = if cfg!(target_os = "macos") {
            "Renamed.app"
        } else {
            "Renamed.exe"
        };
        fs::write(
            tmp.path().join(crate::app_contract::MANIFEST),
            manifest(entry).to_string(),
        )
        .unwrap();
        assert_eq!(
            resolve(tmp.path(), None, Some(&first))
                .unwrap()
                .entry
                .as_deref(),
            Some(entry)
        );
        fs::write(tmp.path().join(crate::app_contract::MANIFEST), "invalid").unwrap();
        // A broken explicit app contract must never silently build a workspace binary.
        assert!(resolve(tmp.path(), None, Some(&legacy)).is_err());
    }
}
