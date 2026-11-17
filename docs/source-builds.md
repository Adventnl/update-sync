# Source build configuration

Install and Update always resolve the current `main` commit before downloading source. A saved configuration selects how to build it. It never pins future updates to an old commit. Detection reads manifests from that exact snapshot and does not use AI.

## Explicit application builds

The preferred repository contract is root `relay.app.json`. It names the app's exact build steps, complete output folder, and launch entry for each supported host OS/CPU. Commands can use any installed language toolchain, including languages outside Relay's built-in recipes. A mixed-language app can build its components in sequence and collect their runtime files together.

Use the [Relay app skill](../skills/relay-app/SKILL.md) to prepare repositories. The [contract reference](../skills/relay-app/references/contract.md) specifies all fields, paths, execution variables, and platform behavior; the [schema](../skills/relay-app/references/relay.app.schema.json) supports editor validation. `relay-cli check-build <repository> --target <os-arch>` validates without executing build code.

Explicit per-install configuration wins. Otherwise an app contract takes precedence over saved legacy recipes, `relay.build.json`, and automatic detection. `system: "app"` explicitly selects the repository contract. Its commands and output paths are reloaded on every update. An invalid contract or absent host target stops the build, without falling back to an unrelated package. The output folder is emptied before the declared steps run. Missing output or launch entry stops installation, even if the commands succeed.

`SKILL.md` guides setup; Relay only executes the machine-readable contract and committed build scripts. Command dispatch is deterministic and needs no AI. Reproducibility also depends on toolchain versions, lockfiles, and the repository's build logic.

## Legacy recipe configuration

Relay prefers a desktop framework over its language manifest. A single nested Tauri or Electron application also takes precedence over a root Cargo workspace. Otherwise, a single root recipe wins, or one nested project is selected. Multiple candidates require configuration rather than guessing. Detection scans at most four directory levels and 20,000 entries, skipping dependency, build, test, and example directories.

The configuration form appears after detection or output selection fails. It saves the selected build system and paths for subsequent updates. To share the same configuration with other installations, a repository may include `relay.build.json` at its root:

```json
{
  "system": "cargo",
  "project": "apps/desktop",
  "target": "desktop",
  "entry": "desktop.exe",
  "resources": ["assets"]
}
```

`system` is one of `cargo`, `tauri`, `electron`, `cmake`, `maven`, `gradle`, `python`, `go`, or `dotnet`. `project` defaults to `.` and is relative to the repository root. Other paths are relative to the selected project. `entry` is relative to the final application output. Omit optional fields to use automatic behavior.

| Field | Meaning |
| --- | --- |
| `target` | Cargo binary name, Python source file, Go package folder, or .NET project file. Other recipes use their standard target. |
| `output` | Folder produced by the build containing the application and its runtime files. Overrides automatic output collection. |
| `entry` | Launch file within the final output. A missing configured entry stops the update. |
| `resources` | Additional project files or directories to copy into the application with their relative paths preserved. |

Paths must remain inside managed directories. Legacy recipe configuration cannot provide shell commands or command-line switches; use the app contract for custom builds. Unknown fields are rejected. Saved legacy configuration overrides `relay.build.json` and detection, while `relay.app.json` takes precedence over those saved choices. Without an explicit entry, Relay reuses the active version's launch path, matches the repository name when unique, or asks for a selection if several candidates remain.

## Recipe behavior

Cargo uses `--release`, a private target directory, and `--locked` when a lockfile exists. It collects executables and native libraries from the release folder. Projects needing additional runtime assets can declare resource paths.

Tauri installs locked Node dependencies and runs the project's declared CLI. The Windows recipe collects the executable, native libraries, JSON-configured bundle resources, and declared sidecars. macOS uses the application bundle produced by Tauri. Non-JSON Windows bundle configurations need an explicit output folder.

Electron runs a declared `build` script when present, then electron-builder in directory mode. It copies the entire unpacked application, including its runtime resources. Repositories must declare electron-builder as a project dependency.

CMake configures and builds Release, then uses the project's install rules with a private prefix. A project without install rules needs a configured output folder.

Maven and Gradle use committed wrappers when present, build with tests skipped and collect JARs whose manifests declare `Main-Class`. Applications must include their dependencies in the application output. Java libraries without a runnable entry are not offered as apps. Java is required to open a JAR.

Python creates a private virtual environment, installs declared requirements and the project when applicable, then uses the pinned PyInstaller recipe on `main.py` or the configured source entry. It retains the complete application folder. Additional runtime assets need resource paths.

Go uses `go build -mod=readonly -trimpath`, targeting the module root or configured package folder. Module and compilation caches are private.

.NET publishes a self-contained application for the current Windows or macOS architecture. Windows retains the full publish output. macOS needs a project that produces a `.app` and a configured output folder containing it.

## Toolchains and recovery

Packaged Relay includes Node.js 24, npm, and npx, with the official distribution's licenses. The packaging hook verifies pinned archive hashes before including the matching Windows/macOS CPU distribution. The Windows portable ZIP includes this toolchain alongside the executable, just as the installer does. Node 24 requires macOS 13.5 or later.

Builds search the packaged Node directory first, then absolute inherited PATH entries, configured SDK locations, common user tool directories (including Cargo and Volta), and standard system/Homebrew locations. This also applies to child build scripts, so opening Relay from Finder does not hide an installed npm or Cargo. Relay does not source shell startup files or modify the user's environment. Development and CLI builds without packaged resources use installed tools.

The runner sets `CI=true`. Tauri's CLI parses this as a boolean and rejects `CI=1`, including when invoked indirectly through an npm packaging script. Regression coverage checks this value along with packaged tool selection and child-process lookup.

Rust, Xcode command line tools, Windows C++ Build Tools, other SDKs, pnpm/Yarn, and system development libraries remain host prerequisites. For example, Athenaeum's app contract uses npm for its JavaScript packages and Cargo for its native app. Relay does not silently install global toolchains. Missing tools produce an error with a retry action. Build systems outside the legacy recipes can use an app contract. Repositories requiring submodules need self-contained source snapshots.

Verified on macOS arm64 on 2026-11-17: all 37 Rust tests, Clippy with warnings denied, frontend checks (15 tests), and the macOS app package passed. Packaged npm/npx and an `npm ci` lifecycle script ran with a minimal desktop PATH. A live install of `Hanoryx-North/athenaeum` at `780f1ae84a2da60b9a428c7ec649c0dcf1e148f8` compiled, passed integrity checks, activated, and removed its temporary workspace. Windows and Intel macOS packages were not executed locally.

Build commands run under the user account with network access. Managed workspaces limit clutter and prevent writes through configured paths outside those folders. They are not a security sandbox for repository code.

The installed application stays active throughout source download and compilation. Failed builds, cancelled builds, and failed activation keep that active version and its data. Activation failures leave prepared output available for retry. Successful activation retains one rollback version and removes older versions. Relay does not test an application's behavior or reverse its data migrations. Restore a previous version through App details if a new build has a runtime problem.
