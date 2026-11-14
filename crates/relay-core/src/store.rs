use crate::model::*;
use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub struct Store {
    pub root: PathBuf,
    db: Connection,
}

impl Store {
    pub fn open(root: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(root)?;
        let root = root.canonicalize()?;
        for name in ["apps", "staging", "builds"] {
            fs::create_dir_all(safe_child(&root, name)?)?;
        }
        let db = Connection::open(safe_child(&root, "relay.db")?)?;
        db.busy_timeout(std::time::Duration::from_secs(15))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS apps (id TEXT PRIMARY KEY, data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS activity (id TEXT PRIMARY KEY, at TEXT NOT NULL, data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), data TEXT NOT NULL);
            PRAGMA user_version=1;")?;
        Ok(Self { root, db })
    }
    pub fn apps(&self) -> anyhow::Result<Vec<App>> {
        let mut stmt = self.db.prepare("SELECT data FROM apps ORDER BY id")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn app(&self, id: &str) -> anyhow::Result<App> {
        let data: Option<String> = self
            .db
            .query_row("SELECT data FROM apps WHERE id=?", [id], |r| r.get(0))
            .optional()?;
        serde_json::from_str(&data.context("This app is no longer in your library.")?)
            .map_err(Into::into)
    }
    pub fn save(&mut self, app: &App, action: &str, detail: &str) -> anyhow::Result<()> {
        let tx = self.db.transaction()?;
        tx.execute("INSERT INTO apps(id,data) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data", params![app.id, serde_json::to_string(app)?])?;
        if !action.is_empty() {
            let event = Activity {
                id: uuid::Uuid::new_v4().to_string(),
                app_id: app.id.clone(),
                app_name: app.name.clone(),
                action: action.into(),
                detail: detail.into(),
                at: now(),
            };
            tx.execute(
                "INSERT INTO activity(id,at,data) VALUES(?,?,?)",
                params![event.id, event.at, serde_json::to_string(&event)?],
            )?;
            tx.execute("DELETE FROM activity WHERE id NOT IN (SELECT id FROM activity ORDER BY at DESC LIMIT 500)", [])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn activity(&self) -> anyhow::Result<Vec<Activity>> {
        let mut stmt = self
            .db
            .prepare("SELECT data FROM activity ORDER BY at DESC LIMIT 200")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn settings(&self) -> anyhow::Result<Settings> {
        let data: Option<String> = self
            .db
            .query_row("SELECT data FROM settings WHERE id=1", [], |r| r.get(0))
            .optional()?;
        data.map(|s| serde_json::from_str(&s))
            .transpose()
            .map(|v| {
                v.unwrap_or(Settings {
                    use_github_cli: true,
                    check_on_open: true,
                    ..Default::default()
                })
            })
            .map_err(Into::into)
    }
    pub fn set_settings(&self, settings: &Settings) -> anyhow::Result<()> {
        self.db.execute("INSERT INTO settings(id,data) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data", [serde_json::to_string(settings)?])?;
        Ok(())
    }
    pub fn app_dir(&self, id: &str) -> anyhow::Result<PathBuf> {
        anyhow::ensure!(
            id.starts_with("gh-") && id[3..].bytes().all(|c| c.is_ascii_digit()) && id.len() > 3,
            "Invalid application identity."
        );
        safe_child(&self.root, &format!("apps/{id}"))
    }
    pub fn version_dir(&self, id: &str, version: &str) -> anyhow::Result<PathBuf> {
        uuid::Uuid::parse_str(version)?;
        safe_child(&self.app_dir(id)?, &format!("versions/{version}"))
    }
}

/// Reject both path traversal and redirects through existing symlinks or junctions.
pub fn safe_child(root: &Path, relative: &str) -> anyhow::Result<PathBuf> {
    let relative = Path::new(relative);
    anyhow::ensure!(
        !relative.as_os_str().is_empty()
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "Unsafe relative path."
    );
    let mut result = root.to_owned();
    for component in relative.components() {
        result.push(component);
        if let Ok(meta) = fs::symlink_metadata(&result) {
            anyhow::ensure!(
                !is_link(&meta),
                "A managed path was replaced with a link: {}",
                result.display()
            );
        }
    }
    Ok(result)
}

pub fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_type().is_symlink() || meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_traversal() {
        let d = tempfile::tempdir().unwrap();
        for p in ["../outside", "/absolute", "a/../../b", ""] {
            assert!(safe_child(d.path(), p).is_err());
        }
        assert!(safe_child(d.path(), "apps/gh-1").is_ok());
    }
    #[test]
    fn settings_and_app_database_survive_reopen() {
        let d = tempfile::tempdir().unwrap();
        let store = Store::open(d.path()).unwrap();
        store
            .set_settings(&Settings {
                use_github_cli: false,
                check_on_open: true,
                appearance: Appearance::Dark,
            })
            .unwrap();
        drop(store);
        assert!(
            !Store::open(d.path())
                .unwrap()
                .settings()
                .unwrap()
                .use_github_cli
        );
    }
}
