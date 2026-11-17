//! Build tools must also be discoverable when Relay is opened from the desktop.
use anyhow::{Context, Result};
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) fn build_path(launcher: &Path) -> Result<OsString> {
    let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_owned());
    let mut paths = Vec::new();
    if let Some(bin) = bundled_node(launcher) {
        paths.push(bin);
    }
    if let Some(inherited) = env::var_os("PATH") {
        // Relative PATH entries would search untrusted source folders first.
        paths.extend(env::split_paths(&inherited).filter(|path| path.is_absolute()));
    }
    for name in ["CARGO_HOME", "VOLTA_HOME", "JAVA_HOME", "GOROOT"] {
        if let Some(root) = env::var_os(name) {
            paths.push(PathBuf::from(root).join("bin"));
        }
    }
    if let Some(home) = home {
        for folder in [
            ".cargo/bin",
            ".volta/bin",
            ".local/bin",
            ".local/share/mise/shims",
            ".asdf/shims",
        ] {
            paths.push(home.join(folder));
        }
        #[cfg(windows)]
        paths.push(home.join("AppData/Roaming/npm"));
    }
    #[cfg(unix)]
    paths.extend(
        [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ]
        .map(PathBuf::from),
    );
    #[cfg(windows)]
    {
        for name in ["ProgramFiles", "LOCALAPPDATA"] {
            if let Some(root) = env::var_os(name) {
                paths.push(PathBuf::from(root).join("nodejs"));
            }
        }
        if let Some(root) = env::var_os("SystemRoot") {
            paths.push(PathBuf::from(root).join("System32"));
        }
    }
    let mut unique = Vec::new();
    for path in paths {
        if path.is_absolute() && path.is_dir() && !unique.contains(&path) {
            unique.push(path);
        }
    }
    env::join_paths(unique).context("The build tool search path is invalid.")
}

fn bundled_node(launcher: &Path) -> Option<PathBuf> {
    let parent = launcher.parent()?;
    let root =
        if cfg!(target_os = "macos") && parent.file_name().is_some_and(|name| name == "MacOS") {
            parent.parent()?.join("Resources/toolchains/node")
        } else {
            parent.join("toolchains/node")
        };
    node_bin(&root)
}

fn node_bin(root: &Path) -> Option<PathBuf> {
    let bin = if cfg!(windows) {
        root.to_owned()
    } else {
        root.join("bin")
    };
    bin.join(if cfg!(windows) { "node.exe" } else { "node" })
        .is_file()
        .then_some(bin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_node_precedes_host_tools_and_paths_are_absolute() {
        let temp = tempfile::tempdir().unwrap();
        let parent = if cfg!(target_os = "macos") {
            temp.path().join("Relay.app/Contents/MacOS")
        } else {
            temp.path().to_owned()
        };
        std::fs::create_dir_all(&parent).unwrap();
        let resources = if cfg!(target_os = "macos") {
            parent.parent().unwrap().join("Resources")
        } else {
            parent.clone()
        };
        let root = resources.join("toolchains/node");
        let bin = if cfg!(windows) {
            root
        } else {
            root.join("bin")
        };
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(
            bin.join(if cfg!(windows) { "node.exe" } else { "node" }),
            "fixture",
        )
        .unwrap();
        let path = build_path(&parent.join("relay")).unwrap();
        let paths: Vec<_> = env::split_paths(&path).collect();
        assert_eq!(paths[0], bin);
        assert!(paths.iter().all(|path| path.is_absolute()));
        assert_eq!(paths.iter().filter(|path| **path == bin).count(), 1);
    }

    #[test]
    fn absent_bundle_falls_back_to_installed_tools() {
        let temp = tempfile::tempdir().unwrap();
        assert!(bundled_node(&temp.path().join("relay")).is_none());
        assert!(!build_path(&temp.path().join("relay")).unwrap().is_empty());
    }
}
