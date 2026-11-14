use crate::{
    manager::{Manager, Prepared},
    model::*,
    platform::Integration,
    store::Store,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

struct TestIntegration {
    fail: AtomicBool,
}
impl Integration for TestIntegration {
    fn verify(&self, _: &Path, _: Option<&str>) -> anyhow::Result<String> {
        Ok("Test signature".into())
    }
    fn register(&self, _: &Store, _: &App, _: &Path) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.fail.swap(false, Ordering::SeqCst),
            "Simulated shortcut failure."
        );
        Ok(())
    }
    fn unregister(&self, _: &Store, _: &App, _: &Path) -> anyhow::Result<()> {
        Ok(())
    }
}
pub(crate) fn app() -> App {
    App {
        id: "gh-1".into(),
        repository_id: 1,
        repository: Repository {
            owner: "fixture".into(),
            name: "Example".into(),
        },
        name: "Example".into(),
        description: "Test fixture".into(),
        stacks: vec!["Rust".into()],
        channel: Channel::Stable,
        tracked: true,
        active_version: None,
        versions: vec![],
        available: None,
        checked_at: None,
        check_error: None,
        desktop_shortcut: false,
        data_folders: vec![],
        build_config: None,
    }
}
fn setup() -> (tempfile::TempDir, Manager, Arc<TestIntegration>) {
    let temp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(
        temp.path().to_owned(),
        PathBuf::from("relay"),
        Arc::new(|_| {}),
    )
    .unwrap();
    let integration = Arc::new(TestIntegration {
        fail: AtomicBool::new(false),
    });
    m.integration = integration.clone();
    m.store()
        .unwrap()
        .save(&app(), "Added", "Test fixture")
        .unwrap();
    (temp, m, integration)
}
fn prepare(m: &Manager, label: &str) -> Prepared {
    let store = m.store().unwrap();
    let app = store.app("gh-1").unwrap();
    let tmp = tempfile::tempdir_in(m.root.join("staging")).unwrap();
    let content = tmp.path().join("content");
    fs::create_dir(&content).unwrap();
    fs::write(content.join("Example.jar"), label).unwrap();
    let package = Package {
        id: label.into(),
        name: "Example.jar".into(),
        url: "https://api.github.com/fixture".into(),
        version: label.into(),
        commit: None,
        digest: None,
        size: label.len() as u64,
        source: "release".into(),
        published_at: now(),
        compatible: true,
    };
    m.persist_prepared(
        &app,
        package,
        "fixture-hash".into(),
        &content,
        vec!["Example.jar".into()],
    )
    .unwrap()
}
#[test]
fn install_update_restore_and_stop_tracking_preserve_versions() {
    let (_temp, m, _) = setup();
    let first = prepare(&m, "v1");
    let first = m.commit(&first.ticket, "Example.jar").unwrap();
    let original = first.active_version.unwrap();
    let second = prepare(&m, "v2");
    let second = m.commit(&second.ticket, "Example.jar").unwrap();
    assert_eq!(second.versions.len(), 2);
    m.track("gh-1", false).unwrap();
    assert!(m
        .store()
        .unwrap()
        .version_dir("gh-1", &original)
        .unwrap()
        .exists());
    m.rollback("gh-1", &original).unwrap();
    let restored = m.store().unwrap().app("gh-1").unwrap();
    assert_eq!(restored.active().unwrap().label, "v1");
    assert!(!restored.tracked);
}
#[test]
fn failed_registration_keeps_previous_version_and_retryable_download() {
    let (_temp, m, integration) = setup();
    let first = prepare(&m, "v1");
    m.commit(&first.ticket, "Example.jar").unwrap();
    let second = prepare(&m, "v2");
    integration.fail.store(true, Ordering::SeqCst);
    assert!(m.commit(&second.ticket, "Example.jar").is_err());
    assert_eq!(
        m.store()
            .unwrap()
            .app("gh-1")
            .unwrap()
            .active()
            .unwrap()
            .label,
        "v1"
    );
    assert!(m
        .root
        .join("staging")
        .join(&second.ticket)
        .join("content/Example.jar")
        .exists());
    m.commit(&second.ticket, "Example.jar").unwrap();
    assert_eq!(
        m.store()
            .unwrap()
            .app("gh-1")
            .unwrap()
            .active()
            .unwrap()
            .label,
        "v2"
    );
}
#[test]
fn changed_version_cannot_be_restored_and_unowned_files_are_not_removed() {
    let (_temp, m, _) = setup();
    let first = prepare(&m, "v1");
    let app = m.commit(&first.ticket, "Example.jar").unwrap();
    let id = app.active_version.unwrap();
    let root = m.store().unwrap().version_dir("gh-1", &id).unwrap();
    fs::write(root.join("Example.jar"), "tampered").unwrap();
    assert_eq!(m.verify("gh-1").unwrap(), vec!["Example.jar"]);
    assert!(m.rollback("gh-1", &id).is_err());
    fs::remove_file(root.join(".relay-owned")).unwrap();
    assert!(m.uninstall("gh-1", true).is_err());
    assert!(root.join("Example.jar").exists());
}
#[test]
fn cleanup_protects_current_and_one_rollback_version() {
    let (_temp, m, _) = setup();
    for label in ["v1", "v2", "v3"] {
        let p = prepare(&m, label);
        m.commit(&p.ticket, "Example.jar").unwrap();
    }
    let list = m.storage().unwrap();
    assert!(list.is_empty());
    let app = m.store().unwrap().app("gh-1").unwrap();
    assert_eq!(app.active().unwrap().label, "v3");
    assert_eq!(app.versions[1].label, "v2");
    assert_eq!(
        fs::read_dir(m.store().unwrap().app_dir("gh-1").unwrap().join("versions"))
            .unwrap()
            .count(),
        2
    );
    assert_eq!(m.store().unwrap().app("gh-1").unwrap().versions.len(), 2);
}
#[test]
fn source_commits_drive_updates_without_release_assets() {
    let repo = app().repository;
    let first = crate::github::source_package(&repo, &"a".repeat(40)).unwrap();
    let second = crate::github::source_package(&repo, &"b".repeat(40)).unwrap();
    let mut installed = InstalledVersion {
        id: "fixture".into(),
        label: first.version.clone(),
        commit: first.commit.clone(),
        installed_at: now(),
        entry: "Example.exe".into(),
        package_digest: "hash".into(),
        source: first.id,
        files: vec![],
        size: 1,
    };
    assert!(
        crate::manager::select_update(std::slice::from_ref(&second), Some(&installed)).is_some()
    );
    installed.commit = second.commit.clone();
    assert!(crate::manager::select_update(&[second], Some(&installed)).is_none());
    assert!(crate::github::source_package(&repo, "main").is_err());
}

#[test]
fn recovery_deletes_only_owned_build_caches_and_keeps_prepared_content() {
    let (_temp, m, _) = setup();
    let prepared = prepare(&m, "v1");
    let builds = m.root.join("builds");
    fs::create_dir_all(builds.join("build-crashed/cache")).unwrap();
    fs::write(builds.join("build-crashed/.relay-owned"), "build").unwrap();
    fs::write(builds.join("build-crashed/cache/dependency"), "cache").unwrap();
    fs::create_dir(builds.join("unowned")).unwrap();
    m.recover().unwrap();
    assert!(!builds.join("build-crashed").exists());
    assert!(builds.join("unowned").is_dir());
    assert!(m
        .root
        .join("staging")
        .join(prepared.ticket)
        .join("content")
        .exists());
}

#[test]
fn actual_cargo_source_compiles_and_failed_rebuild_keeps_data_and_active_version() {
    use crate::recipes::{BuildConfig, BuildSystem};
    let (_temp, m, _) = setup();
    let work = tempfile::tempdir_in(m.root.join("builds")).unwrap();
    let project = work.path().join("source");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='source-fixture'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    fs::write(
        project.join("src/main.rs"),
        "fn main() { println!(\"built from source\"); }",
    )
    .unwrap();
    let environment = work.path().join("environment");
    fs::create_dir(&environment).unwrap();
    let output = work.path().join("package");
    fs::create_dir(&output).unwrap();
    // Common build scripts concatenate OUT_DIR with '/', including on Windows.
    fs::write(project.join("build.rs"), r#"fn main() { std::fs::write(format!("{}/generated.rs", std::env::var("OUT_DIR").unwrap()), "").unwrap(); }"#).unwrap();
    let config = BuildConfig::new(BuildSystem::Cargo, ".".into());
    m.compile_project("gh-1", &project, &environment, &output, "gh-1", &config)
        .unwrap();
    let entries = crate::archive::entries(&output).unwrap();
    assert_eq!(entries.len(), 1);
    let mut package = crate::github::source_package(&app().repository, &"a".repeat(40)).unwrap();
    package.source = "local".into();
    let prepared = m
        .persist_prepared(
            &app(),
            package,
            "source-digest".into(),
            &output,
            entries.clone(),
        )
        .unwrap();
    let installed = m.commit(&prepared.ticket, &entries[0]).unwrap();
    assert_eq!(
        installed.active().unwrap().commit.as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    let data = m.store().unwrap().app_dir("gh-1").unwrap().join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("settings.json"), "keep").unwrap();
    fs::write(project.join("src/main.rs"), "this does not compile").unwrap();
    fs::create_dir(&output).unwrap();
    assert!(m
        .compile_project("gh-1", &project, &environment, &output, "gh-1", &config)
        .is_err());
    assert_eq!(
        m.store().unwrap().app("gh-1").unwrap().active_version,
        installed.active_version
    );
    assert_eq!(
        fs::read_to_string(data.join("settings.json")).unwrap(),
        "keep"
    );
    assert!(m.verify("gh-1").unwrap().is_empty());
    work.close().unwrap();
    assert!(fs::read_dir(m.root.join("builds"))
        .unwrap()
        .next()
        .is_none());
}
#[test]
fn interrupted_promotion_is_recovered_without_changing_active_version() {
    let (_temp, m, _) = setup();
    let p = prepare(&m, "v1");
    m.commit(&p.ticket, "Example.jar").unwrap();
    let pending = prepare(&m, "v2");
    let stage = m.root.join("staging").join(&pending.ticket);
    let id = uuid::Uuid::new_v4().to_string();
    let v = InstalledVersion {
        id: id.clone(),
        label: "v2".into(),
        commit: None,
        installed_at: now(),
        entry: "Example.jar".into(),
        package_digest: "hash".into(),
        source: "v2".into(),
        files: vec![],
        size: 2,
    };
    fs::write(
        stage.join("promotion.json"),
        serde_json::to_vec(&v).unwrap(),
    )
    .unwrap();
    fs::write(stage.join("content/.relay-owned"), format!("gh-1:{id}")).unwrap();
    let target = m.store().unwrap().version_dir("gh-1", &id).unwrap();
    fs::rename(stage.join("content"), &target).unwrap();
    m.recover().unwrap();
    assert_eq!(
        m.store()
            .unwrap()
            .app("gh-1")
            .unwrap()
            .active()
            .unwrap()
            .label,
        "v1"
    );
    assert!(stage.join("content/Example.jar").exists());
    assert!(!target.exists());
}
#[test]
fn concurrent_operations_are_refused() {
    let (_temp, m, _) = setup();
    let _guard = m.lock().unwrap();
    assert!(m.track("gh-1", false).is_err());
}

#[test]
fn another_asset_in_the_same_release_is_not_an_update() {
    let (_temp, m, _) = setup();
    let prepared = prepare(&m, "release-msvc");
    let app = m.commit(&prepared.ticket, "Example.jar").unwrap();
    let mut installed = app.active().unwrap().clone();
    installed.label = "v1".into();
    installed.entry = "app-windows-msvc/app.exe".into();
    let package = |id: &str, version: &str, when: &str| Package {
        id: id.into(),
        name: format!("{id}.zip"),
        url: "https://api.github.com/fixture".into(),
        version: version.into(),
        commit: None,
        digest: None,
        size: 1,
        source: "release".into(),
        published_at: when.into(),
        compatible: true,
    };
    let same = [
        package("release-gnu", "v1", "date-1"),
        package("release-msvc", "v1", "date-1"),
    ];
    assert!(crate::manager::select_update(&same, Some(&installed)).is_none());
    let newer = [
        package("new-gnu", "v2", "date-2"),
        package("new-msvc", "v2", "date-2"),
        same[1].clone(),
    ];
    assert_eq!(
        crate::manager::select_update(&newer, Some(&installed))
            .unwrap()
            .id,
        "new-msvc"
    );
}
