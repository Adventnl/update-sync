# Relay

A small desktop library for your GitHub applications. Built with Rust, Tauri, React, and TypeScript for Windows and macOS. No AI services, telemetry, or background daemon.

Paste a GitHub repository URL and click Install. Relay resolves the latest `main` commit, detects the build system from that exact source snapshot, compiles it, and activates the application. Update repeats the same process with the latest commit. No AI, release packages, GitHub Actions workflow, or manual build commands are needed.

Builds run with your account permissions in disposable, managed folders. Install repositories you trust. The relevant compiler or SDK must already be installed. Relay discovers the installed Windows C++ compiler and SDK without requiring a developer terminal.

## Run

Running packaged Relay does not require Node or Rust. Compiling an application requires its toolchain. On Windows, the locally built installer is `target/release/bundle/nsis/Relay_0.1.0_x64-setup.exe`.

For development, install Node.js 24+, the current stable Rust toolchain, and [Tauri's platform prerequisites](https://v2.tauri.app/start/prerequisites/). Windows needs Microsoft C++ Build Tools and WebView2. macOS needs Xcode command line tools.

```sh
npm ci
npm run desktop
```

Use `npm run desktop:build` to build platform packages. The packaging workflow builds Windows x64 and macOS arm64/x64 artifacts when manually dispatched. It does not publish a release or change another repository.

## Source builds

| Build system | Automatic recipe |
| --- | --- |
| Rust / Cargo | Release build, committed lockfile when present, executable and native libraries |
| Tauri | Locked Node dependencies, project Tauri CLI, native application and declared resources |
| Electron | Locked Node dependencies, project build script when present, electron-builder unpacked application |
| CMake | Release configure, compile, and install into a private output directory |
| Maven / Gradle | Build executable JARs with a Main-Class manifest |
| Python | Private environment and PyInstaller application from main.py or a configured entry |
| Go | Native binary with read-only module resolution |
| .NET | Self-contained publish for the current platform and architecture |

Node projects need a committed npm, pnpm, or Yarn lockfile and the corresponding package manager. Tauri and Electron build CLIs must be project dependencies. Java applications need the Java runtime when opened. .NET macOS projects need an application bundle output path. Build recipes target the current operating system and CPU architecture.

Relay checks root manifests and nested projects up to four folders deep. Framework manifests take precedence over their implementation language. Ambiguous layouts open a configuration form. Choose the build system, project folder, and any target, output folder, launch file, or resources once. Relay saves those choices for future updates. A chosen launch file is reused when it exists in the next build. See [source build configuration](docs/source-builds.md) for recipe details and optional repository-owned configuration.

Repositories must have a `main` branch. Source snapshots do not include Git submodule contents. Custom toolchains, platform signing requirements, projects needing privileged installers, and unsupported build systems may need project changes. Relay reports a failure while keeping the installed version active. It cannot infer arbitrary packaging steps.

## GitHub access

Relay uses an existing `gh auth login` session by default. Settings can disable it or save a separate token in the OS credential store. Public repositories work without sign-in, subject to GitHub rate limits. Source builds need Metadata and Contents read permissions. Legacy release and workflow commands remain available in the CLI. Workflow commands require Actions permissions. Organization SSO restrictions may require authorization by your organization.

## Safety and data

Updates are staged, hashed, inventoried, and journaled before activation. Relay automatically keeps the current version and one rollback version, then removes obsolete versions after successful activation. Running applications are left alone; close them before updating or removing them. An application's data migrations may make older versions incompatible with newer settings.

Uninstall removes owned versions and shortcuts. User data is kept by default. Register an app-specific data folder in App details to include it in optional complete removal. Relay does not guess ownership of shared or unrelated files. Stop tracking changes only the tracking record. Apps and shortcuts continue to work, and pasting the same repository link reconnects them.

Source builds use private dependency caches that are removed after success, failure, or cancellation. Startup removes owned interrupted builds. Each build command has a 30-minute time limit, a 32 MB log limit, and a periodically enforced 12 GB workspace limit. Cargo incremental output is disabled and compilation uses at most four jobs. Storage can remove remaining old versions, interrupted workspaces, and explicitly selected old Rust/CMake caches. It never clears global toolchain caches or scans an entire disk automatically.

## Command line

The Rust engine is available without the GUI:

```sh
cargo run -p relay-cli -- list
cargo run -p relay-cli -- inspect https://github.com/owner/app --main
cargo run -p relay-cli -- install https://github.com/owner/app
cargo run -p relay-cli -- update gh-123
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
