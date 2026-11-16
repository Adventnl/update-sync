---
name: relay-app
description: Prepare a repository for Relay to compile and install its desktop app without AI. Use when adding or fixing relay.app.json, packaging a mixed-language app, or writing app-specific build setup instructions.
---

# Prepare a Relay app

Make the repository describe how to build one complete runnable desktop app. Relay reads `relay.app.json` at the repository root and executes its declared steps; it never reads this skill at build time. Any language or combination of languages can use the contract. The result must be a Windows app, macOS bundle, or runnable Java archive.

Read [the contract](references/contract.md) for the format, command environment, and platform behavior. Use [the JSON schema](references/relay.app.schema.json) when validating a manifest.

## Adapt the actual app

- Inspect the repository's client, build scripts, lockfiles, resources, and platform packaging. Identify the app the user wants to open. A workspace's server, library, helper, or first binary is not necessarily that app.
- Keep existing source folders when they work. The only fixed source location is root `relay.app.json`. Add a small packaging script if the existing build does not produce a complete app folder.
- Declare exact steps for each supported host OS and CPU. Use existing compilers, wrappers, and pinned dependencies; do not choose recipes from a language allowlist. For an unfamiliar language, determine its actual build and runtime packaging commands from the repository and toolchain documentation.
- Put the finished app and all runtime files into a dedicated disposable output folder. Relay deletes this folder before building, so it must contain no source, configuration, or user data. Use `RELAY_OUTPUT_DIR` inside scripts. Respect `CARGO_TARGET_DIR` and other runner-provided cache/output paths rather than assuming `target/` under the source tree.
- Declare the launch entry relative to that output. Include required DLLs, frameworks, sidecars, assets, interpreter/runtime files, and licenses. An installed app must not depend on Relay's temporary source or dependency caches. An executable JAR requires `Main-Class` and packaged dependencies; Java itself is provided by the machine.
- Build only. Do not launch the app, start a VPN, register a service, change networking, install drivers, or deploy a server as a packaging step. Apps needing those installation actions need separate integration; a build contract does not supply it.

For a Duct Tape VPN-style repository, select the desktop client explicitly, build its native bridge and UI as needed, then collect the client bundle and runtime files together. A successful root Cargo workspace build alone does not establish that the desktop client was built. Treat mobile clients and server deployment as separate targets outside Relay's desktop installation contract.

## Verify and hand off

Run `relay-cli check-build /path/to/repo --target <os-arch>` for each declared target when Relay CLI is available. It checks structure and paths without executing commands or proving the toolchain is installed. Run the declared steps with the [documented environment](references/contract.md#command-environment), starting with an empty output folder, on each available platform. Check the actual entry, runtime files, and relocation away from source/cache folders. Then test Install and Update through Relay when the repository is available on GitHub `main`.

If the user requests an app-specific `SKILL.md`, place it in the requested location or `.agents/skills/relay-build/SKILL.md` in that app repository. Include its exact app target, real packaging commands, required toolchains, manifest location, and verified checks. Describe untested targets as untested in technical documentation; do not claim verification or invent missing packaging steps. Relay uses JSON and scripts; prose instructions alone cannot make a repository buildable.
