use clap::{Parser, Subcommand};
use relay_core::*;
use std::{path::PathBuf, sync::Arc};

#[derive(Parser)]
#[command(
    about = "Relay application manager. Every desktop lifecycle operation is available here."
)]
struct Args {
    #[arg(long, alias = "library")]
    root: Option<PathBuf>,
    #[arg(long)]
    launcher: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Validate a repository app contract without running build code.
    CheckBuild {
        path: PathBuf,
        /// For example: windows-x86_64 or macos-aarch64. Defaults to this machine.
        #[arg(long)]
        target: Option<String>,
    },
    List,
    /// Compile main and install it. Configuration is saved for future updates.
    Install {
        url: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Compile the latest main commit for an existing app.
    Update {
        id: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Settings {
        #[arg(long)]
        appearance: Option<String>,
        #[arg(long)]
        check_on_open: Option<bool>,
        #[arg(long)]
        use_github_cli: Option<bool>,
    },
    Channel {
        id: String,
        #[arg(long)]
        main: bool,
    },
    Workflows {
        id: String,
    },
    RequestBuild {
        id: String,
        workflow: u64,
    },
    Prepared,
    RegisterData {
        id: String,
        path: PathBuf,
    },
    ForgetData {
        id: String,
        path: PathBuf,
    },
    Reveal {
        id: Option<String>,
        #[arg(long)]
        data: bool,
    },
    Repository {
        id: String,
    },
    Inspect {
        url: String,
        #[arg(long)]
        main: bool,
    },
    Add {
        url: String,
        #[arg(long)]
        main: bool,
    },
    Check,
    Prepare {
        id: String,
        package: String,
        #[arg(long)]
        allow_unverified: bool,
    },
    Commit {
        ticket: String,
        entry: String,
    },
    Launch {
        id: String,
    },
    Uninstall {
        id: String,
        #[arg(long)]
        remove_owned_data: bool,
    },
    Track {
        id: String,
    },
    Untrack {
        id: String,
    },
    Rollback {
        id: String,
        version: String,
    },
    Verify {
        id: String,
    },
    Shortcuts {
        id: String,
        #[arg(long)]
        desktop: bool,
    },
    Storage,
    Cleanup {
        ids: Vec<String>,
    },
    ScanCaches {
        root: PathBuf,
    },
    RemoveCache {
        root: PathBuf,
        path: PathBuf,
    },
    BuildPlan {
        id: String,
    },
    Build {
        id: String,
        #[arg(long)]
        trust_code: bool,
    },
    Discard {
        ticket: String,
    },
}
fn main() -> anyhow::Result<()> {
    let raw = std::env::args().map(|arg| match arg.as_str() {
        "--launch" => "launch".into(),
        "--uninstall" => "uninstall".into(),
        _ => arg,
    });
    let args = Args::parse_from(raw);
    if let Command::CheckBuild { path, target } = &args.command {
        let path = std::fs::canonicalize(path)?;
        let selected = app_contract::load_for(
            &path,
            target.as_deref().unwrap_or(&app_contract::current_target()),
        )?;
        println!("{}", serde_json::to_string_pretty(&selected)?);
        return Ok(());
    }
    let root = args
        .root
        .map(Ok)
        .unwrap_or_else(relay_core::platform::default_root)?;
    let launcher = args.launcher.unwrap_or(std::env::current_exe()?);
    let m = Manager::new(
        root,
        launcher,
        Arc::new(|p| eprintln!("{}: {}", p.stage, p.message)),
    )?;
    m.recover()?;
    let value = match args.command {
        Command::CheckBuild { .. } => unreachable!(),
        Command::List => serde_json::to_value(m.snapshot()?)?,
        Command::Install { url, config } => {
            let app = m.add(&url, Channel::Main)?;
            install_source(&m, &app.id, config)?
        }
        Command::Update { id, config } => install_source(&m, &id, config)?,
        Command::Settings {
            appearance,
            check_on_open,
            use_github_cli,
        } => {
            let mut settings = m.store()?.settings()?;
            if let Some(value) = appearance {
                settings.appearance = match value.as_str() {
                    "system" => Appearance::System,
                    "light" => Appearance::Light,
                    "dark" => Appearance::Dark,
                    _ => anyhow::bail!("Appearance must be system, light, or dark."),
                };
            }
            if let Some(value) = check_on_open {
                settings.check_on_open = value;
            }
            if let Some(value) = use_github_cli {
                settings.use_github_cli = value;
            }
            m.store()?.set_settings(&settings)?;
            serde_json::to_value(settings)?
        }
        Command::Channel { id, main } => {
            m.channel(&id, if main { Channel::Main } else { Channel::Stable })?;
            serde_json::Value::Null
        }
        Command::Workflows { id } => serde_json::to_value(m.workflows(&id)?)?,
        Command::RequestBuild { id, workflow } => {
            m.request_build(&id, workflow)?;
            serde_json::Value::Null
        }
        Command::Prepared => serde_json::to_value(m.prepared_installs()?)?,
        Command::RegisterData { id, path } => {
            m.register_data_folder(&id, &path)?;
            serde_json::Value::Null
        }
        Command::ForgetData { id, path } => {
            m.forget_data_folder(&id, &path)?;
            serde_json::Value::Null
        }
        Command::Reveal { id, data } => {
            m.reveal(id.as_deref(), data)?;
            serde_json::Value::Null
        }
        Command::Repository { id } => {
            m.open_repository(&id)?;
            serde_json::Value::Null
        }
        Command::Inspect { url, main } => serde_json::to_value(
            m.inspect(&url, &if main { Channel::Main } else { Channel::Stable })?,
        )?,
        Command::Add { url, main } => {
            serde_json::to_value(m.add(&url, if main { Channel::Main } else { Channel::Stable })?)?
        }
        Command::Prepare {
            id,
            package,
            allow_unverified,
        } => serde_json::to_value(m.prepare(&id, &package, allow_unverified)?)?,
        Command::Commit { ticket, entry } => serde_json::to_value(m.commit(&ticket, &entry)?)?,
        Command::Check => {
            m.check_updates()?;
            serde_json::Value::Null
        }
        Command::Launch { id } => {
            m.launch(&id)?;
            serde_json::Value::Null
        }
        Command::Uninstall {
            id,
            remove_owned_data,
        } => {
            m.uninstall(&id, remove_owned_data)?;
            serde_json::Value::Null
        }
        Command::Track { id } => {
            m.track(&id, true)?;
            serde_json::Value::Null
        }
        Command::Untrack { id } => {
            m.track(&id, false)?;
            serde_json::Value::Null
        }
        Command::Rollback { id, version } => {
            m.rollback(&id, &version)?;
            serde_json::Value::Null
        }
        Command::Verify { id } => serde_json::to_value(m.verify(&id)?)?,
        Command::Shortcuts { id, desktop } => {
            m.shortcuts(&id, desktop)?;
            serde_json::Value::Null
        }
        Command::Storage => serde_json::to_value(m.storage()?)?,
        Command::Cleanup { ids } => serde_json::to_value(m.cleanup(&ids)?)?,
        Command::ScanCaches { root } => serde_json::to_value(m.scan_caches(&root)?)?,
        Command::RemoveCache { root, path } => serde_json::to_value(m.remove_cache(&root, &path)?)?,
        Command::BuildPlan { id } => serde_json::to_value(m.build_plan(&id)?)?,
        Command::Build { id, trust_code } => serde_json::to_value(m.local_build(&id, trust_code)?)?,
        Command::Discard { ticket } => {
            m.discard(&ticket)?;
            serde_json::Value::Null
        }
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn install_source(
    m: &Manager,
    id: &str,
    path: Option<PathBuf>,
) -> anyhow::Result<serde_json::Value> {
    let config = path
        .map(|path| -> anyhow::Result<recipes::BuildConfig> {
            Ok(serde_json::from_slice(&std::fs::read(path)?)?)
        })
        .transpose()?;
    let prepared = m.build_source(id, config)?;
    if prepared.entries.len() == 1 {
        Ok(serde_json::to_value(
            m.commit(&prepared.ticket, &prepared.entries[0])?,
        )?)
    } else {
        // The caller can select an entry with the existing commit command once.
        Ok(serde_json::to_value(prepared)?)
    }
}
