# Relay app contract, version 1

Commit `relay.app.json` at the repository root. It describes a complete application rather than a package or language. Source folders may keep their existing layout. A suggested shape is:

```text
relay.app.json
apps/desktop/             # Optional; any existing project folder works
scripts/package-app.*    # Optional; project-owned build/collection logic
dist/relay-app/           # Disposable finished output; ignore in Git
```

## Manifest

```json
{
  "version": 1,
  "project": ".",
  "targets": {
    "windows-x86_64": {
      "steps": [
        { "program": "powershell.exe", "args": ["-NoProfile", "-File", "scripts/package-app.ps1"] }
      ],
      "output": "dist/relay-app",
      "entry": "Duct Tape VPN.exe"
    },
    "macos-aarch64": {
      "steps": [
        { "program": "sh", "args": ["scripts/package-app.sh"] }
      ],
      "output": "dist/relay-app",
      "entry": "Duct Tape VPN.app"
    }
  }
}
```

The example names packaging scripts you must implement for the actual app; it is not a ready-made Duct Tape VPN build.

| Field | Meaning |
| --- | --- |
| `version` | Required integer `1`. Unknown versions fail. |
| `project` | Project folder relative to the repository root; defaults to `.`. |
| `targets` | One or more exact host targets: `windows-x86_64`, `windows-aarch64`, `macos-x86_64`, `macos-aarch64`. Declare only those the app supports. |
| `steps` | Between 1 and 64 commands, run sequentially from `project`. A nonzero exit stops the build. |
| `program` | Tool name from `PATH`, or project-relative executable path containing `/`, such as `tools/builder.exe`. No absolute paths or `./` prefix. Generated executables are allowed in later steps. |
| `args` | Optional array of individual arguments, at most 256 per step. Each argument is passed literally, with no variable, glob, or template expansion. Use an explicit script interpreter for script logic. |
| `output` | Required project-relative folder with the entire runnable app and runtime files. Relay removes it before the first step, creates it empty, then copies its full contents into the installed version. Never point this at source folders. |
| `entry` | Required path relative to `output`: `.exe` on Windows, `.app` on macOS, or executable `.jar` on either platform. |

Paths use `/` on every platform and cannot contain absolute paths, drive letters, backslashes, empty segments, `.` segments, `..`, or control characters. `project: "."` is the sole exception. Paths are limited to 500 bytes. Program names are limited to 500 bytes, arguments to 16,384 bytes each, and the manifest to 256 KB. Unknown fields are rejected. The selected project must exist; output and generated programs need not exist until the commands run.

Relay chooses the exact current OS/CPU pair. It does not cross-compile or fall back to a different architecture. An absent target produces a configuration error rather than guessing. Language names and manifests are not consulted for app contracts.

## Command environment

Relay executes commands with a reduced environment, the user's permissions, and network access. Packaged Relay supplies Node.js 24, npm, and npx. Other compilers, SDKs, system development libraries, and script interpreters must already be installed. The contract cannot silently provision host toolchains. Dependency installation can be an explicit build step, for example `npm ci`; commit and use the package manager's lockfile.

| Variable | Meaning |
| --- | --- |
| `RELAY_PROJECT_DIR` | Absolute selected project folder. |
| `RELAY_OUTPUT_DIR` | Absolute empty output folder for this target. Packaging scripts should place the finished application here. |
| `RELAY_PLATFORM` | `windows` or `macos`. |
| `RELAY_ARCH` | `x86_64` or `aarch64`. |
| `CARGO_TARGET_DIR` | Private compiler output. Use this instead of assuming a source-local `target` folder. |
| `CARGO_HOME`, Node/Python/Gradle/Go/.NET cache variables, `TEMP`/`TMP`/`TMPDIR`, `XDG_CACHE_HOME` | Disposable build environment and dependency caches, removed after the build. |

`PATH` starts with Relay's packaged Node directory, followed by absolute inherited entries and common user/system tool locations. Home paths and selected SDK locations such as `JAVA_HOME`, `DOTNET_ROOT`, `RUSTUP_HOME`, `SDKROOT`, and `DEVELOPER_DIR` are inherited. Arbitrary environment variables and Relay's GitHub token are not forwarded. Configure other language-specific settings in committed scripts or explicit command arguments. There is no shell expansion in `args`; `$RELAY_OUTPUT_DIR` as an argument stays that literal string. Read environment variables inside scripts. On Windows, known package-manager commands are invoked through `cmd.exe` and reject shell metacharacters in arguments; use a committed script for more elaborate logic.

For a manual script check, first remove and recreate only the declared disposable output folder, set the four `RELAY_*` variables above to absolute paths/current host values, and run the declared steps from `project`. Also set a temporary `CARGO_TARGET_DIR` when relevant. This catches packaging scripts that assume a source-local target folder.

Every command uses Relay's existing cancellation, process-tree termination, 30-minute time limit, 32 MB log limit, and monitored 12 GB workspace budget. Build code is trusted repository code, not an OS sandbox. Tool versions and lockfiles determine reproducibility; a deterministic command dispatcher alone does not guarantee identical bytes.

## Finished application

Relay checks the explicit entry even if its name contains words such as `Helper` or `Installer`. It does not select among workspace binaries. Include the app's runtime assets in `output`; build workspaces are deleted, so references to them will break after installation. Settings and user data belong outside the installed version folder.

macOS bundles need `Contents/Info.plist` and `Contents/MacOS`. Relative symlinks are preserved only on macOS, must resolve within `output`, and cannot be dangling; absolute links are rejected because installed output is relocated. Windows output cannot contain links. A valid macOS signature is preserved; otherwise Relay attempts its existing ad-hoc signing for local builds. Apps that require particular entitlements or distribution signing should supply a correctly signed bundle. Normal platform checks still apply at activation.

Java archives must have a `Main-Class` manifest and include their application dependencies. Relay runs them with the installed Java runtime. The engine currently installs desktop launch formats; support for any language does not imply support for Android/iOS packages, Linux apps, privileged installers, services, or drivers.

## Resolution and checks

An explicit per-install build configuration wins. Otherwise `relay.app.json` wins over old saved recipe configuration, `relay.build.json`, and manifest detection. Relay reloads the contract from the exact new `main` commit on every update. An invalid contract stops the build; it never falls back to compiling an unrelated workspace package. Choosing `system: "app"` in an explicit config also reloads all fields from the repository contract.

```sh
relay-cli check-build /path/to/repository --target windows-x86_64
relay-cli check-build /path/to/repository --target macos-aarch64
```

From Relay's checkout, use `cargo run -p relay-cli -- check-build ...`. This command reads and validates without installing or running build code. It reports the selected project, commands, output, and launch entry. Passing validation does not prove the commands succeed, the toolchain exists, or the app runs. Test those on the target platform. A failed build or missing entry leaves the installed version active.
