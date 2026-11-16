# Minimal desktop redesign — 2026-11-16

Relay now uses one app list with a single Add app action. The sidebar, global Activity screen, search, category navigation, repeated add controls, slogans, and redundant build descriptions were removed. Updates appear beside their app; paused apps remain in the list. Settings and Storage are available from the gear menu. Installation, app details, recovery, tracking, cleanup, and cancellation retain their existing service calls.

The initial window is 820×640 with a 640×500 minimum. macOS hides the duplicate native title. Native window appearance follows resolved light/dark settings and system changes. DESIGN.md and .impeccable/design.json describe the implementation.

## Verification

- Type checking, lint, production frontend build, and 15 tests passed.
- Native macOS release bundle built successfully and replaced ~/Applications/Relay.app.
- Final installed main-window capture: .impeccable/review/desktop.png.
- Tests exercise install/update/recovery, settings/storage navigation, menu accessibility clicks, and resuming a paused app.
- Native UI inspection confirmed repository entry, settings, storage, and the minimum window during development. Final main-window capture confirms the hidden native title and minimal layout.

## Visual verification limits

The independent final review returned `recapture`: the install capture was mid-animation, storage retained a native menu overlay, and settings/minimum captures preceded the final bundle. Native automation intermittently failed with noWindowsAvailable and cgWindowNotFound, preventing reliable final recaptures. Source review confirmed the requested storage/copy/documentation fixes; no full visual approval is claimed. Populated rows, light appearance, and Windows remain visually unverified. No real repository installation was run during this frontend task.
