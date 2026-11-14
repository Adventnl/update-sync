#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use relay_core::{build::BuildPlan, manager::Prepared, storage::StorageItem, *};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::{Emitter, Manager as TauriManager};

type Response<T> = Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Response<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(error)?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn snapshot(manager: tauri::State<'_, relay_core::Manager>) -> Response<Snapshot> {
    let m = manager.inner().clone();
    blocking(move || m.snapshot()).await
}
#[tauri::command]
async fn inspect_repository(
    manager: tauri::State<'_, relay_core::Manager>,
    url: String,
    channel: Channel,
) -> Response<Inspection> {
    let m = manager.inner().clone();
    blocking(move || m.inspect(&url, &channel)).await
}
#[tauri::command]
async fn add_app(
    manager: tauri::State<'_, relay_core::Manager>,
    url: String,
    channel: Channel,
) -> Response<App> {
    let m = manager.inner().clone();
    blocking(move || m.add(&url, channel)).await
}
#[tauri::command]
async fn prepare_install(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
    package_id: String,
    allow_unverified: bool,
) -> Response<Prepared> {
    let m = manager.inner().clone();
    blocking(move || m.prepare(&id, &package_id, allow_unverified)).await
}
#[tauri::command]
async fn commit_install(
    manager: tauri::State<'_, relay_core::Manager>,
    ticket: String,
    entry: String,
) -> Response<App> {
    let m = manager.inner().clone();
    blocking(move || m.commit(&ticket, &entry)).await
}
#[tauri::command]
async fn local_build(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
    trust: bool,
) -> Response<Prepared> {
    let m = manager.inner().clone();
    blocking(move || m.local_build(&id, trust)).await
}
#[tauri::command]
async fn build_source(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
    configuration: Option<relay_core::recipes::BuildConfig>,
) -> Response<Prepared> {
    let m = manager.inner().clone();
    blocking(move || m.build_source(&id, configuration)).await
}
#[tauri::command]
async fn build_plan(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
) -> Response<BuildPlan> {
    let m = manager.inner().clone();
    blocking(move || m.build_plan(&id)).await
}
#[tauri::command]
async fn storage_items(
    manager: tauri::State<'_, relay_core::Manager>,
) -> Response<Vec<StorageItem>> {
    let m = manager.inner().clone();
    blocking(move || m.storage()).await
}
#[tauri::command]
async fn scan_caches(
    manager: tauri::State<'_, relay_core::Manager>,
    root: String,
) -> Response<Vec<StorageItem>> {
    let m = manager.inner().clone();
    blocking(move || m.scan_caches(&PathBuf::from(root))).await
}
#[tauri::command]
async fn remove_cache(
    manager: tauri::State<'_, relay_core::Manager>,
    root: String,
    path: String,
) -> Response<u64> {
    let m = manager.inner().clone();
    blocking(move || m.remove_cache(&PathBuf::from(root), &PathBuf::from(path))).await
}
#[tauri::command]
async fn cleanup(
    manager: tauri::State<'_, relay_core::Manager>,
    ids: Vec<String>,
) -> Response<u64> {
    let m = manager.inner().clone();
    blocking(move || m.cleanup(&ids)).await
}
#[tauri::command]
async fn save_settings(
    manager: tauri::State<'_, relay_core::Manager>,
    settings: Settings,
) -> Response<()> {
    let m = manager.inner().clone();
    blocking(move || m.store()?.set_settings(&settings)).await
}
#[tauri::command]
async fn save_token(token: String) -> Response<()> {
    blocking(move || relay_core::github::set_token(&token)).await
}
#[tauri::command]
fn cancel_operation(manager: tauri::State<'_, relay_core::Manager>) {
    manager
        .cancelled
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
async fn workflows(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
) -> Response<Vec<Workflow>> {
    let m = manager.inner().clone();
    blocking(move || m.workflows(&id)).await
}
#[tauri::command]
async fn prepared_installs(
    manager: tauri::State<'_, relay_core::Manager>,
) -> Response<Vec<Prepared>> {
    let m = manager.inner().clone();
    blocking(move || m.prepared_installs()).await
}
#[tauri::command]
async fn register_data(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
    path: String,
) -> Response<()> {
    let m = manager.inner().clone();
    blocking(move || m.register_data_folder(&id, &PathBuf::from(path))).await
}
#[tauri::command]
async fn forget_data(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
    path: String,
) -> Response<()> {
    let m = manager.inner().clone();
    blocking(move || m.forget_data_folder(&id, &PathBuf::from(path))).await
}
#[tauri::command]
async fn request_build(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
    workflow: u64,
) -> Response<()> {
    let m = manager.inner().clone();
    blocking(move || m.request_build(&id, workflow)).await
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Action {
    CheckUpdates,
    Launch {
        id: String,
    },
    Track {
        id: String,
        tracked: bool,
    },
    Channel {
        id: String,
        channel: Channel,
    },
    Shortcuts {
        id: String,
        desktop: bool,
    },
    Rollback {
        id: String,
        version: String,
    },
    Uninstall {
        id: String,
        #[serde(rename = "removeData")]
        remove_data: bool,
    },
    Reveal {
        id: Option<String>,
        data: bool,
    },
    Repository {
        id: String,
    },
    Discard {
        ticket: String,
    },
}
#[tauri::command]
async fn perform(manager: tauri::State<'_, relay_core::Manager>, action: Action) -> Response<()> {
    let m = manager.inner().clone();
    blocking(move || {
        match action {
            Action::CheckUpdates => m.check_updates()?,
            Action::Launch { id } => m.launch(&id)?,
            Action::Track { id, tracked } => m.track(&id, tracked)?,
            Action::Channel { id, channel } => m.channel(&id, channel)?,
            Action::Shortcuts { id, desktop } => m.shortcuts(&id, desktop)?,
            Action::Rollback { id, version } => m.rollback(&id, &version)?,
            Action::Uninstall { id, remove_data } => m.uninstall(&id, remove_data)?,
            Action::Reveal { id, data } => m.reveal(id.as_deref(), data)?,
            Action::Repository { id } => m.open_repository(&id)?,
            Action::Discard { ticket } => m.discard(&ticket)?,
        };
        Ok(())
    })
    .await
}
#[tauri::command]
async fn verify_install(
    manager: tauri::State<'_, relay_core::Manager>,
    id: String,
) -> Response<Vec<String>> {
    let m = manager.inner().clone();
    blocking(move || m.verify(&id)).await
}

fn main() {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let library = if args.first().is_some_and(|a| a == "--library") && args.len() >= 2 {
        let path = PathBuf::from(args.remove(1));
        args.remove(0);
        Some(path)
    } else {
        None
    };
    if args.len() == 2 && matches!(args[0].as_str(), "--launch" | "--uninstall") {
        let result = (|| -> anyhow::Result<()> {
            let m = relay_core::Manager::new(
                library
                    .clone()
                    .map(Ok)
                    .unwrap_or_else(relay_core::platform::default_root)?,
                std::env::current_exe()?,
                Arc::new(|_| {}),
            )?;
            m.recover()?;
            if args[0] == "--launch" {
                m.launch(&args[1])?;
            } else {
                m.uninstall(&args[1], false)?;
            }
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("{e}");
            rfd::MessageDialog::new()
                .set_title("Relay couldn't finish this action")
                .set_description(format!(
                    "{e:#}\n\nOpen Relay for installation details or repair."
                ))
                .set_level(rfd::MessageLevel::Error)
                .show();
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if let Some(manager) = window.try_state::<relay_core::Manager>() {
                    if manager.operation_active() {
                        api.prevent_close();
                        manager
                            .cancelled
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        manager.report(
                            "",
                            "stopping",
                            "Stopping the current operation. Close Relay again when it finishes.",
                        );
                    }
                }
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            let manager = relay_core::Manager::new(
                library
                    .clone()
                    .map(Ok)
                    .unwrap_or_else(relay_core::platform::default_root)?,
                std::env::current_exe()?,
                Arc::new(move |progress| {
                    let _ = handle.emit("operation-progress", progress);
                }),
            )?;
            manager.recover()?;
            app.manage(manager);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            inspect_repository,
            add_app,
            prepare_install,
            commit_install,
            local_build,
            build_source,
            build_plan,
            storage_items,
            scan_caches,
            remove_cache,
            cleanup,
            save_settings,
            save_token,
            cancel_operation,
            perform,
            verify_install,
            workflows,
            request_build,
            prepared_installs,
            register_data,
            forget_data
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            rfd::MessageDialog::new()
                .set_title("Relay couldn't start")
                .set_description(error.to_string())
                .set_level(rfd::MessageLevel::Error)
                .show();
            std::process::exit(1);
        });
}
