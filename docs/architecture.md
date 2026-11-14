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

1. Re-fetch repository identity and package metadata from GitHub. The UI supplies a package ID, never an arbitrary download URL.
2. Download into a disposable folder with a 4 GB limit and verify GitHub's SHA-256 digest when available. Missing digests require explicit approval.
3. Extract under an 8 GB and 100,000-file limit. Reject traversal, devices, hard links, reserved names, duplicate files, and unsafe links. On macOS, internal relative symlinks are resolved and checked.
4. Persist a prepared-install plan. A package with several executable candidates requires selection. Completed downloads can be resumed from Storage.
5. Check signatures, inventory files, flush file contents, and persist a promotion journal before moving the prepared content into a new version folder.
6. Register launch integration and commit the new active-version pointer in SQLite. Keep previous versions for rollback.
7. Remove staging data. A failed integration or metadata commit restores the old integration and returns the new content to staging. Startup recovers interrupted promotions by consulting the journal and active pointer.

Filesystem and OS registration cannot participate in one SQLite transaction. Recovery is compensating, with a durable journal. A broken external registrar or storage device can still require repair. Relay checks for running executables before changing or removing app versions and does not terminate them.

## Platform integration

Windows uses per-user Start menu and optional Desktop shortcuts. Shortcuts call Relay's stable launcher with the library and application ID. User-level Installed Apps entries launch Relay's uninstaller. Existing shortcuts and registry entries must match their expected owner before mutation.

macOS installs verified `.app` bundles into managed version directories and exposes a stable symlink in `~/Applications/Relay`. Downloaded bundles must pass Gatekeeper and signature checks. Explicitly trusted local builds are ad-hoc signed and verified; the OS still decides whether they can launch. Relay does not edit the Dock database; users can pin an app normally. Existing Dock aliases and platform launch behavior require verification on an actual Mac.

## Builds

Stable packages come from non-draft, non-prerelease GitHub Releases. Main-channel artifacts come from successful push, scheduled, or manually dispatched workflows on main, or the default branch when main is absent. Fork pull-request runs are excluded. Expired artifacts are ignored. GitHub Actions dispatch is available for existing workflows with default inputs.

Local fallback downloads a pinned source snapshot, presents a deterministic stack-specific plan, and requires explicit trust. Commands use argument vectors rather than repository-provided shell strings. They run with a reduced environment and disposable dependency caches. Builds have a 30-minute and 32 MB log limit. Cancellation stops the process tree on Windows and the process group on Unix. The workspace is not an OS security sandbox. Trusted code still has the user's account permissions and network access.

## Cleanup

Managed cleanup protects the active version and one recent alternate. It rechecks ownership and current metadata under the mutation lock. Interrupted downloads and local-build workspaces have explicit store markers. Development scans require a chosen root, ignore links and source-control/dependency folders, and recognize Rust target folders and CMake caches. Every cache file must be more than a day old. Running build tools and processes in the project block removal. Shared toolchain caches are never swept.
