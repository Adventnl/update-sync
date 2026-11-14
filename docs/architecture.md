# Architecture

Relay has a Rust lifecycle engine, a thin Tauri IPC adapter, a command-line adapter, and a React interface. The interface cannot run shell commands or access arbitrary files. Network traffic and credentials stay in Rust. Tauri capabilities expose only local commands, events, and a native folder picker.

## Identity and persistence

GitHub's numeric repository ID is the application identity, such as `gh-53631945`. Owner and repository names can change without creating a second installation. SQLite stores app records, settings, version receipts, and a bounded activity history. WAL mode and full synchronization protect metadata commits. A cross-process file lock serializes lifecycle mutations. Read-only library queries use independent database connections.

The application data directory contains:

```
relay.db
apps/<repository-id>/versions/<version-uuid>/
apps/<repository-id>/data/
staging/<ticket-uuid>/plan.json
staging/<ticket-uuid>/content/
builds/<disposable-workspace>/
```

Each installed version has a store-written ownership marker and a receipt containing relative paths, file hashes, package digest, source asset ID, version, commit when available, and install time. User data stays outside version replacement. External app data can be explicitly registered with its own ownership marker.

## Installation transaction

1. Resolve the repository's numeric identity and full `main` commit SHA. Release and Actions endpoints are not part of source installation or source update checks.
2. Download the source archive pinned to that commit, subject to the existing 4 GB download and 8 GB extraction limits. Record its SHA-256 digest.
3. Detect a reusable build recipe from the extracted manifests, or use a saved configuration or `relay.build.json`. Run it in a disposable managed workspace with private dependency caches. Only finished application files enter staging.
4. Persist a prepared-install plan. Reuse a saved launch entry or select the sole candidate. Several unmatched candidates require a choice once. Completed builds can be resumed from Storage.
5. Check platform integrity, inventory files, flush file contents, and persist a promotion journal before moving the prepared content into a new version folder.
6. Register launch integration and commit the new active-version pointer in SQLite. Keep the previous version for rollback.
7. Remove staging data and obsolete versions. A failed integration or metadata commit restores the old integration and returns the new content to staging. Startup recovers interrupted promotions by consulting the journal and active pointer.

Filesystem and OS registration cannot participate in one SQLite transaction. Recovery is compensating, with a durable journal. A broken external registrar or storage device can still require repair. Relay checks for running executables before changing or removing app versions and does not terminate them.

## Platform integration

Windows uses per-user Start menu and optional Desktop shortcuts. Shortcuts call Relay's stable launcher with the library and application ID. User-level Installed Apps entries launch Relay's uninstaller. Existing shortcuts and registry entries must match their expected owner before mutation.

macOS installs verified `.app` bundles into managed version directories and exposes a stable symlink in `~/Applications/Relay`. Downloaded bundles must pass Gatekeeper and signature checks. Explicitly trusted local builds are ad-hoc signed and verified; the OS still decides whether they can launch. Relay does not edit the Dock database; users can pin an app normally. Existing Dock aliases and platform launch behavior require verification on an actual Mac.

## Builds

`recipes.rs` detects Cargo, npm/pnpm/Yarn Tauri and Electron projects, CMake, Maven, Gradle, Python, Go, and .NET. Detection runs against the pinned source, not stale stored language labels. Missing or ambiguous manifests return a configuration request. Saved configuration contains only a recipe enum, paths, and a target. It cannot inject custom shell commands. The CLI exposes the same source operations through `install` and `update`. Legacy release APIs remain for compatibility.

Commands run with a reduced environment and disposable dependency caches. Windows compiler and SDK paths are discovered through `cc::windows_registry`. Builds have a 30-minute limit per command and a 32 MB log limit. The runner checks workspace size every two seconds and stops above 12 GB. This is a monitored budget, not a disk quota, so short-lived writes can exceed it. Cargo incremental builds are disabled; native parallel builds use at most four workers. The Windows runner assigns each command to a kill-on-close Job Object. Cancellation also stops the process tree on Windows and the process group on Unix. The workspace is not an OS security sandbox. Repository code has the user's account permissions and network access.

The build workspace is removed on success, failure, or cancellation. Owned crash leftovers are removed at startup or before the next build under the operation lock. Prepared output and promotion journals are preserved for retry. No global package or toolchain cache is swept.

## Cleanup

Activation automatically prunes obsolete versions while keeping the active version and one recent alternate. Managed cleanup protects those versions. It rechecks ownership and current metadata under the mutation lock. Interrupted downloads and local-build workspaces have explicit store markers. Development scans require a chosen root, ignore links and source-control/dependency folders, and recognize Rust target folders and CMake caches. Every cache file must be more than a day old. Running build tools and processes in the project block removal. Shared toolchain caches are never swept.
