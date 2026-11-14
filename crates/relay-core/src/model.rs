use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub owner: String,
    pub name: String,
}

impl Repository {
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        let value = input.trim().trim_end_matches('/');
        let url = url::Url::parse(value)?;
        anyhow::ensure!(
            url.scheme() == "https" && url.host_str() == Some("github.com"),
            "Paste an HTTPS github.com repository URL."
        );
        anyhow::ensure!(
            url.username().is_empty()
                && url.password().is_none()
                && url.port().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "Use a plain repository URL without credentials, query, or fragment."
        );
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        anyhow::ensure!(
            parts.len() == 2,
            "Use the repository URL, without a branch or file path."
        );
        let owner = parts[0];
        let name = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
        for part in [owner, name] {
            anyhow::ensure!(
                !part.is_empty()
                    && part != "."
                    && part != ".."
                    && part.len() <= 100
                    && part
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c)),
                "The repository name is invalid."
            );
        }
        Ok(Self {
            owner: owner.into(),
            name: name.into(),
        })
    }
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
    pub fn url(&self) -> String {
        format!("https://github.com/{}", self.slug())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Main,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub id: String,
    pub name: String,
    pub url: String,
    pub version: String,
    pub commit: Option<String>,
    pub digest: Option<String>,
    pub size: u64,
    pub source: String,
    pub published_at: String,
    pub compatible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub repository: Repository,
    pub repository_id: u64,
    pub description: String,
    pub default_branch: String,
    pub head_commit: Option<String>,
    pub stacks: Vec<String>,
    pub packages: Vec<Package>,
    pub warnings: Vec<String>,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledVersion {
    pub id: String,
    pub label: String,
    pub commit: Option<String>,
    pub installed_at: String,
    pub entry: String,
    pub package_digest: String,
    pub source: String,
    pub files: Vec<OwnedFile>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    pub id: String,
    pub repository_id: u64,
    pub repository: Repository,
    pub name: String,
    pub description: String,
    pub stacks: Vec<String>,
    pub channel: Channel,
    pub tracked: bool,
    pub active_version: Option<String>,
    pub versions: Vec<InstalledVersion>,
    pub available: Option<Package>,
    pub checked_at: Option<String>,
    pub check_error: Option<String>,
    pub desktop_shortcut: bool,
    #[serde(default)]
    pub data_folders: Vec<String>,
}

impl App {
    pub fn active(&self) -> Option<&InstalledVersion> {
        self.versions
            .iter()
            .find(|v| Some(&v.id) == self.active_version.as_ref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub app_id: String,
    pub app_name: String,
    pub action: String,
    pub detail: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub appearance: Appearance,
    pub use_github_cli: bool,
    pub check_on_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub apps: Vec<App>,
    pub activity: Vec<Activity>,
    pub settings: Settings,
    pub root: String,
    pub platform: String,
    pub architecture: String,
    pub auth: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub app_id: String,
    pub stage: String,
    pub message: String,
    pub completed: u64,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
}

pub type Reporter = std::sync::Arc<dyn Fn(Progress) + Send + Sync>;
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_urls_are_strict() {
        assert_eq!(
            Repository::parse("https://github.com/me/app.git/")
                .unwrap()
                .slug(),
            "me/app"
        );
        for url in [
            "https://github.com.evil.test/me/app",
            "https://me:secret@github.com/me/app",
            "http://github.com/me/app",
            "https://github.com/me/app/tree/main",
            "https://github.com/me/%2e%2e",
            "https://github.com/me/app?token=x",
            "https://github.com:444/me/app",
        ] {
            assert!(Repository::parse(url).is_err(), "{url}");
        }
    }
}
