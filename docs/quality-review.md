# Code Cleanup Report — entire project

## Scorecard

| Category | Rating |
| --- | --- |
| Architecture & layering | Pass |
| File size & granularity | Pass |
| DRY & reuse | Pass |
| Hardcoding / magic values | Pass |
| Types & correctness | Pass |
| Async & error handling | Pass |
| State management | Pass |
| Performance & complexity | Pass |
| Density & clarity | Pass |
| Naming & readability | Pass |
| Security | Needs work |
| Tests & quality gates | Needs work |

Overall: The Windows application builds and its implemented lifecycle has automated and live verification. A complete production release still needs signing and native macOS verification. General-purpose privileged installers are deliberately outside the managed package model.

## Findings

### P0 — Critical

- `crates/relay-core/src/archive.rs` requires strict boundaries for untrusted packages. Fixed with path validation, reserved-name checks, extraction limits, duplicate rejection, and internal-link checks.
- `crates/relay-core/src/manager.rs` must preserve the active version on integration failure or interruption. Fixed with staged versions, durable promotion journals, database commits, compensation, and startup recovery. Failure and recovery tests pass.
- `crates/relay-core/src/storage.rs` and `data.rs` must establish ownership before recursive removal. Fixed with revalidated candidates, receipts, markers, path checks, protected versions, and explicit external-data registration. Shared and unregistered folders are kept.

### P1 — High

- `crates/relay-core/src/manager.rs` initially treated another asset in the same release as an update. The live ripgrep test exposed this. Fixed by grouping the newest published build, recognizing the installed source asset, and preserving package flavor. A regression test and repeated live check pass.
- `crates/relay-core/src/scripts/shortcut.ps1` initially compared Windows path spellings literally. The native uninstall test exposed a forward-slash mismatch. Fixed by normalizing paths before comparing ownership. Native tests also confirm another launcher's shortcut is preserved.
- `src/components/Library.tsx` initially relied on pointer activation for the Radix menu. Fixed by handling semantic accessibility clicks. A React regression test covers click activation without pointer events.
- `src/components/Sidebar.tsx` initially lost accessible names when compact navigation hid text. Fixed with persistent button labels, titles, and current-page attributes.
- `src/hooks/StoreProvider.tsx` now refreshes authoritative state after failures, including partially completed removals. Launch and folder actions can run independently of network update checks.

### P2 — Medium

- Local builds support documented stack conventions. Custom layouts, submodules, privileged resources, unusual dependency packaging, and project-specific migrations need their own packaging recipes. Remote compiled packages remain the preferred path.
- The detector reported two motion advisories for the restrained spring curve. The brief explicitly asks for physically natural motion. Reduced-motion overrides are present.

### P3 — Nits

- No open material findings are recorded here for minor copy or formatting. This is a review of the implemented scope, not a guarantee that every future repository can be managed.

## Remediation plan

1. Completed the filesystem, transaction, identity, native registration, and recovery fixes before packaging.
2. Completed the observed update-selection and accessibility corrections with regression coverage.
3. Complete native macOS checks and supply signing credentials before a cross-platform production release. A Windows host cannot establish those results.
4. Add installer adapters only with explicit uninstall identities, owned-resource inventories, and adapter-specific tests. Do not run arbitrary setup files and promise portable-package rollback.

## Verification

- Rust workspace tests cover repository validation, extraction, database reopening, ownership, interrupted promotion, rollback, tracking, version cleanup, release variants, cross-process locks, and native Windows shortcut semantics.
- Frontend tests cover the actual empty state, adding a repository, checksum approval, recoverable errors, storage navigation, system appearance, and semantic menu activation.
- `npm run check` passes TypeScript, ESLint, seven frontend tests, and the production web build.
- `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings` pass on Windows. The core has 20 tests, including two native shortcut tests.
- `npm audit` reports zero known dependency vulnerabilities. The minimatch override removes the reported development-tool advisory; lint and tests verify its compatibility.
- `npm run desktop:build -- --bundles nsis` produces the Windows executable and a roughly 4 MB per-user installer.
- A real ripgrep release was downloaded into an isolated test library. Its 13 files verified, its installed executable ran, Windows registration and a Start menu shortcut were created, tracking changes preserved the files, and uninstall removed its registration and shortcut.
- A second install verified Windows Attachment Services origin metadata and confirmed that the same release did not produce a false update.
- Native desktop captures cover light and dark modes, real installed content, app details, repository review, and the minimum supported window. macOS is not claimed as verified.

Inspected the entire project. Fixed critical filesystem and transaction boundaries, high-priority identity and accessibility defects, and native integration failures. Deferred signing, macOS execution, and unsupported installer adapters for the reasons above. Changed the Rust engine and adapters, React interface, platform packaging, tests, and repository documentation. Added lifecycle, parsing, extraction, native shortcut, and UI flow coverage. Windows gates pass within the stated scope. No unrelated features added.
