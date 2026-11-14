use crate::model::*;
use anyhow::{Context, Result};
use reqwest::{blocking::Client, header};
use serde::Deserialize;
use std::{io::Read, time::Duration};

const API: &str = "https://api.github.com";
const TOKEN_SERVICE: &str = "dev.relay.store.github";

pub struct Github {
    client: Client,
    token: Option<String>,
}

pub fn set_token(token: &str) -> Result<()> {
    let entry = keyring::Entry::new(TOKEN_SERVICE, "personal")?;
    if token.trim().is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    } else {
        anyhow::ensure!(
            !token.contains(['\n', '\r']) && token.len() < 1024,
            "Invalid GitHub token."
        );
        entry
            .set_password(token.trim())
            .context("Could not save the token to your operating system credential store.")
    }
}

impl Github {
    pub fn new(use_cli: bool) -> Result<Self> {
        let stored = keyring::Entry::new(TOKEN_SERVICE, "personal").and_then(|e| e.get_password());
        let token = match stored {
            Ok(token) => Some(token),
            Err(keyring::Error::NoEntry) => None,
            Err(e) => anyhow::bail!("Cannot read the GitHub credential store: {e}. Unlock it or remove the saved token."),
        };
        let token = token.or_else(|| {
            if !use_cli {
                return None;
            }
            let output = crate::platform::command("gh")
                .args(["auth", "token", "--hostname", "github.com"])
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .filter(|s| !s.is_empty())
        });
        let client = Client::builder()
            .user_agent("Relay/0.1.0")
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(90))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 8 || attempt.url().scheme() != "https" {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .build()?;
        Ok(Self { client, token })
    }
    pub fn auth_label(&self) -> String {
        if self.token.is_some() {
            "Connected to GitHub"
        } else {
            "Public access"
        }
        .into()
    }
    pub fn workflows(&self, repo: &Repository) -> Result<Vec<Workflow>> {
        #[derive(Deserialize)]
        struct Workflows {
            workflows: Vec<Workflow>,
        }
        let result: Workflows = self.get(&format!(
            "/repos/{}/actions/workflows?per_page=100",
            repo.slug()
        ))?;
        Ok(result
            .workflows
            .into_iter()
            .filter(|w| w.state == "active")
            .collect())
    }
    pub fn dispatch(&self, repo: &Repository, workflow: u64) -> Result<()> {
        anyhow::ensure!(
            self.token.is_some(),
            "Sign in to GitHub with Actions write permission to request a build."
        );
        let info: RepoInfo = self.get(&format!("/repos/{}", repo.slug()))?;
        let url = format!(
            "{API}/repos/{}/actions/workflows/{workflow}/dispatches",
            repo.slug()
        );
        let mut request = self
            .client
            .post(url)
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header(header::ACCEPT, "application/vnd.github+json")
            .json(&serde_json::json!({"ref":info.default_branch}));
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        let response = request.send()?;
        anyhow::ensure!(response.status().is_success(),"GitHub could not start this workflow (HTTP {}). The workflow needs workflow_dispatch, default inputs, and your token needs Actions write access.",response.status().as_u16());
        Ok(())
    }
    fn request(&self, url: &str) -> reqwest::blocking::RequestBuilder {
        let request = self
            .client
            .get(url)
            .header("X-GitHub-Api-Version", "2022-11-28");
        match &self.token {
            Some(t) => request.bearer_auth(t),
            None => request,
        }
    }
    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .request(&format!("{API}{path}"))
            .header(header::ACCEPT, "application/vnd.github+json")
            .send()
            .context("GitHub could not be reached. Check your connection and try again.")?;
        let status = response.status();
        if !status.is_success() {
            let reset = response
                .headers()
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("soon")
                .to_owned();
            return Err(match status.as_u16() {
                401 => anyhow::anyhow!("GitHub sign-in expired. Update your token in Settings."),
                403 | 429 => anyhow::anyhow!("GitHub denied this request or its rate limit was reached. Check token permissions and try later. Rate limit resets: {reset}."),
                404 => anyhow::anyhow!("Repository or build not found. Private repositories need a token with Contents and Actions read access."),
                _ => anyhow::anyhow!("GitHub returned HTTP {}. Try again shortly.", status.as_u16()),
            });
        }
        response
            .json()
            .context("GitHub returned an unexpected response.")
    }
    pub fn inspect(&self, repo: &Repository, channel: &Channel) -> Result<Inspection> {
        let base = format!("/repos/{}", repo.slug());
        let info: RepoInfo = self.get(&base)?;
        let repo = Repository::parse(&info.html_url)?;
        let base = format!("/repos/{}", repo.slug());
        let mut result = Inspection {
            repository: repo,
            repository_id: info.id,
            description: info.description.unwrap_or_default(),
            default_branch: info.default_branch,
            head_commit: None,
            stacks: vec![],
            packages: vec![],
            warnings: vec![],
            html_url: info.html_url,
        };
        let branch = "main";
        let mut commit_url = url::Url::parse(&format!("{API}{base}/commits"))?;
        commit_url
            .query_pairs_mut()
            .append_pair("sha", branch)
            .append_pair("per_page", "1");
        let commits: Vec<Commit> = self
            .get(commit_url.as_str().strip_prefix(API).unwrap_or_default())
            .context("Could not resolve main. Source installations require a main branch.")?;
        result.head_commit = commits.first().map(|c| c.sha.clone());
        match self.get::<Vec<Content>>(&format!(
            "{base}/contents?ref={}",
            result.head_commit.as_deref().unwrap_or("main")
        )) {
            Ok(entries) => {
                let names: Vec<_> = entries.iter().map(|v| v.name.as_str()).collect();
                for (files, stack) in [
                    (&["Cargo.toml"][..], "Rust"),
                    (&["src-tauri", "tauri.conf.json"][..], "Tauri"),
                    (&["CMakeLists.txt", "meson.build"][..], "C++"),
                    (&["pom.xml", "build.gradle", "build.gradle.kts"][..], "Java"),
                    (
                        &["pyproject.toml", "requirements.txt", "setup.py"][..],
                        "Python",
                    ),
                    (&["package.json"][..], "Node.js"),
                    (&["go.mod"][..], "Go"),
                    (&["Package.swift"][..], "Swift"),
                ] {
                    if files.iter().any(|f| names.contains(f)) {
                        result.stacks.push(stack.into());
                    }
                }
                if names.contains(&"package.json") {
                    if let Ok(response) = self
                        .request(&format!(
                            "{API}{base}/contents/package.json?ref={}",
                            result.head_commit.as_deref().unwrap_or("main")
                        ))
                        .header(header::ACCEPT, "application/vnd.github.raw+json")
                        .send()
                        .and_then(|r| r.error_for_status())
                    {
                        let mut text = String::new();
                        if response.take(1024 * 1024).read_to_string(&mut text).is_ok() {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                if ["dependencies", "devDependencies"].iter().any(|key| {
                                    value.get(key).is_some_and(|v| v.get("electron").is_some())
                                }) {
                                    result.stacks.push("Electron".into());
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => result.warnings.push(e.to_string()),
        }
        match channel {
            Channel::Stable => {
                let releases: Vec<Release> = self.get(&format!("{base}/releases?per_page=30"))?;
                for release in releases.into_iter().filter(|r| !r.draft && !r.prerelease) {
                    for asset in release.assets {
                        if is_package(&asset.name) {
                            result.packages.push(Package {
                                id: format!("release-{}", asset.id),
                                compatible: compatible(&asset.name),
                                name: asset.name,
                                url: asset.url,
                                version: release.tag_name.clone(),
                                commit: None,
                                digest: asset.digest,
                                size: asset.size,
                                source: "release".into(),
                                published_at: release.published_at.clone().unwrap_or_default(),
                            });
                        }
                    }
                }
            }
            Channel::Main => {
                let commit = result
                    .head_commit
                    .clone()
                    .context("The main branch is empty.")?;
                result
                    .packages
                    .push(source_package(&result.repository, &commit)?);
                return Ok(result);
            }
        }

        if result.packages.is_empty() {
            result.warnings.push("No downloadable packages found. Publish a portable ZIP or a macOS app ZIP, use the other channel, or review a local build.".into());
        }
        if result.packages.iter().any(|p| p.digest.is_none()) {
            result.warnings.push("Some packages have no publisher SHA-256 digest. Installing one requires explicit approval; Relay will still record its download hash.".into());
        }
        Ok(result)
    }
    pub fn download(
        &self,
        package: &Package,
        destination: &std::path::Path,
        report: impl Fn(u64, Option<u64>) -> Result<()>,
    ) -> Result<String> {
        validate_download_url(&package.url)?;
        let mut response = self
            .request(&package.url)
            .header(
                header::ACCEPT,
                if package.source == "source" || package.id.starts_with("source-") {
                    "application/vnd.github+json"
                } else {
                    "application/octet-stream"
                },
            )
            .timeout(Duration::from_secs(1800))
            .send()?
            .error_for_status()
            .context("Package download failed. Your existing installation has not changed.")?;
        let total = response.content_length();
        let mut file = std::fs::File::create(destination)?;
        let mut hash = sha2::Sha256::default();
        use sha2::Digest;
        use std::io::Write;
        let mut buffer = [0; 65536];
        let mut completed = 0u64;
        loop {
            let count = response.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            completed += count as u64;
            anyhow::ensure!(
                completed <= 4 * 1024 * 1024 * 1024,
                "The package exceeds Relay's 4 GB download limit."
            );
            file.write_all(&buffer[..count])?;
            hash.update(&buffer[..count]);
            report(completed, total)?;
        }
        file.sync_all()?;
        let digest = hex::encode(hash.finalize());
        if let Some(expected) = &package.digest {
            anyhow::ensure!(expected.strip_prefix("sha256:").unwrap_or(expected).eq_ignore_ascii_case(&digest), "Package verification failed. The downloaded SHA-256 does not match GitHub. Nothing was installed.");
        }
        Ok(digest)
    }
}

/// A source candidate is identified by the full commit, independent of releases and CI.
pub fn source_package(repo: &Repository, commit: &str) -> Result<Package> {
    anyhow::ensure!(
        commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()),
        "GitHub returned an invalid main commit."
    );
    Ok(Package {
        id: format!("source-{commit}"),
        name: "source.zip".into(),
        url: format!(
            "https://api.github.com/repos/{}/zipball/{commit}",
            repo.slug()
        ),
        version: format!("main-{}", &commit[..7]),
        commit: Some(commit.into()),
        digest: None,
        size: 0,
        source: "source".into(),
        published_at: String::new(),
        compatible: true,
    })
}

pub fn validate_download_url(value: &str) -> Result<()> {
    let url = url::Url::parse(value)?;
    anyhow::ensure!(
        url.scheme() == "https"
            && matches!(url.host_str(), Some("api.github.com" | "github.com"))
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none(),
        "Packages must originate from GitHub over HTTPS."
    );
    Ok(())
}
pub fn is_package(name: &str) -> bool {
    let n = name.to_lowercase();
    !n.contains(".sig")
        && !n.contains("checksum")
        && !n.contains("symbols")
        && (n.ends_with(".zip")
            || n.ends_with(".exe")
            || n.ends_with(".dmg")
            || n.ends_with(".jar")
            || n.ends_with(".tar.gz"))
}
pub fn compatible(name: &str) -> bool {
    let name = name.to_lowercase();
    let windows = name.contains("windows")
        || name.contains("win32")
        || name.contains("win64")
        || name.contains("pc-windows")
        || name.ends_with(".exe");
    let mac = name.contains("darwin")
        || name.contains("macos")
        || name.contains("mac-")
        || name.ends_with(".dmg");
    let linux = name.contains("linux") || name.ends_with(".appimage") || name.contains("ubuntu");
    if linux || (cfg!(windows) && mac) || (cfg!(target_os = "macos") && windows) {
        return false;
    }
    if name.contains("arm64") || name.contains("aarch64") {
        return cfg!(target_arch = "aarch64");
    }
    if name.contains("x86_64") || name.contains("amd64") || name.contains("x64") {
        return cfg!(target_arch = "x86_64")
            || (cfg!(target_os = "macos") && name.contains("universal"));
    }
    !name.contains("i686") && !name.contains("x86-32")
}

#[derive(Deserialize)]
struct RepoInfo {
    id: u64,
    description: Option<String>,
    default_branch: String,
    html_url: String,
}
#[derive(Deserialize)]
struct Content {
    name: String,
}
#[derive(Deserialize)]
struct Commit {
    sha: String,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    id: u64,
    name: String,
    url: String,
    digest: Option<String>,
    size: u64,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_foreign_download_origins() {
        assert!(
            validate_download_url("https://api.github.com/repos/me/app/releases/assets/1").is_ok()
        );
        for u in [
            "https://evil.test/a.zip",
            "http://github.com/a",
            "https://github.com@evil.test/a",
            "https://github.com:9000/a",
        ] {
            assert!(validate_download_url(u).is_err());
        }
    }
    #[test]
    fn ignore_foreign_platforms_and_signatures() {
        assert!(!compatible("app-linux-x64.zip"));
        assert!(!is_package("app.zip.sig"));
        assert!(is_package("app-win64.zip"));
        assert!(!compatible("app-i686.zip"));
    }
}
