# Relay

A small desktop library for your GitHub applications. Built with Rust, Tauri, React, and TypeScript for Windows and macOS. No AI services, telemetry, or background daemon.

Paste a repository link, inspect its real packages, install a version, and open it. Relay supports stable releases and artifacts from successful main builds, private repositories, recoverable updates, previous-version restore, repair, stable launch shortcuts, history, stop tracking, and scoped storage cleanup.

## Run

Packaged users do not need Node or Rust. On Windows, the locally built installer is `target/release/bundle/nsis/Relay_0.1.0_x64-setup.exe`.

For development, install Node.js 24+, the current stable Rust toolchain, and [Tauri's platform prerequisites](https://v2.tauri.app/start/prerequisites/). Windows needs Microsoft C++ Build Tools and WebView2. macOS needs Xcode command line tools.

```sh
npm ci
npm run desktop
```

Use `npm run desktop:build` to build platform packages. The packaging workflow builds Windows x64 and macOS arm64/x64 artifacts when manually dispatched. It does not publish a release or change another repository.

## Packages that work

| Package | Windows | macOS |
| --- | --- | --- |
| ZIP / TAR.GZ containing a portable app and its resources | Yes, with EXE entry | Yes, with a signed and notarized .app |
| Portable EXE | Yes | No |
| DMG with one .app | No | Yes, signature and Gatekeeper checked |
| Runnable JAR | Yes, requires Java | Yes, requires Java |
| MSI, setup EXE, PKG, services, drivers | Not managed | Not managed |

Packages work independently of their implementation language. Rust, C++, Java, Python, Electron, Tauri, Go, and other projects can publish portable packages without changing their source architecture. The store cannot turn every repository or arbitrary installer into a safely reversible portable app.

Name artifacts with the platform and architecture, such as `MyApp-windows-x64` or `MyApp-macos-arm64`. Include the executable and required resources, or one ZIP/TAR.GZ/DMG containing the app. Relay unwraps a single nested package automatically and verifies the outer artifact's GitHub digest. Multiple nested packages require a more specific artifact.

Existing workflows can be requested with **Build on GitHub**. They need `workflow_dispatch` with default inputs. Local fallback supports common Cargo, npm Tauri/Electron, CMake, Java, and Python main.py layouts. It requires the relevant toolchain and explicit trust. Local builds use disposable workspaces and dependency caches, not an OS security sandbox. Unusual layouts, missing resources, custom build steps, submodules, and macOS signing need a project-specific remote packaging workflow.

## GitHub access

Relay uses an existing `gh auth login` session by default. Settings can disable it or save a separate token in the OS credential store. Public repositories work without sign-in, subject to GitHub rate limits. Fine-grained tokens need Metadata, Contents, and Actions read permissions. Starting a remote build also needs Actions write permission. Organization SSO restrictions may require authorization by your organization.

## Safety and data

Updates are staged, hashed, inventoried, and journaled before activation. Relay keeps the current version and a rollback version. Running applications are left alone; close them before updating or removing them. An application's data migrations may make older versions incompatible with newer settings.

Uninstall removes owned versions and shortcuts. User data is kept by default. Register an app-specific data folder in App details to include it in optional complete removal. Relay does not guess ownership of shared or unrelated files. Stop tracking changes only the tracking record. Apps and shortcuts continue to work, and pasting the same repository link reconnects them.

Storage can remove old versions, interrupted workspaces, and explicitly selected old Rust/CMake caches. It never clears global toolchain caches or scans an entire disk automatically.

## Command line

The Rust engine is available without the GUI:

```sh
cargo run -p relay-cli -- list
cargo run -p relay-cli -- inspect https://github.com/owner/app
cargo run -p relay-cli -- add https://github.com/owner/app --main
cargo run -p relay-cli -- check
cargo run -p relay-cli -- prepare gh-123 artifact-456
cargo run -p relay-cli -- commit <ticket> path/to/app.exe
cargo run -p relay-cli -- verify gh-123
cargo run -p relay-cli -- storage
```

Use `--root <folder>` for an isolated CLI library and `--launcher <absolute Relay executable>` when creating shortcuts from the CLI. The desktop also accepts `--library <folder>`. `relay-cli --help` lists lifecycle commands.

## Check

```sh
npm run check
npm audit
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Windows builds and engine/React tests have been exercised locally. macOS CI and packaging are configured but have not been run in this Windows session. Signing credentials must be supplied for production distribution. See [architecture](docs/architecture.md) and [security](docs/security.md) for the transaction, trust model, and limits.
