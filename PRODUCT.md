# Relay

<!-- impeccable:product-schema 1 -->

## Platform

web

Desktop application for Windows and macOS using each system's webview, window chrome, typography, keyboard conventions, and file integration. This is not a hosted website.

## Stack

Delegated by the user. Rust owns privileged operations and persistence. Tauri hosts a React and TypeScript interface. No AI services are part of the application.

## Users

One person managing their own GitHub applications across personal Windows and macOS computers.

## Product purpose

Paste a GitHub URL, compile and install its latest main commit, open it, update it safely, and remove resources with known ownership. Keep build and version clutter under control.

## Operating context

Detect build systems deterministically from pinned main source snapshots. Install and Update compile in disposable managed workspaces with private dependency caches. Ask for reusable configuration only when detection is insufficient. Build execution uses the user account and is not an operating-system security sandbox.

## Capabilities and constraints

Stable repository identities, stable launch shortcuts, recoverable version selection, private repository access, source commit tracking, history, repair, stop tracking, and scoped cache cleanup. Never infer ownership of unrelated files or silently terminate running apps. Do not bypass platform signing checks. No administrator privileges by default.

Arbitrary repositories cannot always be packaged automatically. Missing or ambiguous build layouts need a clear explanation and a saved project configuration. Applications requiring system installers, privileged services, or drivers need their native installer and cannot receive the same transaction guarantee as portable packages.

## Brand commitments

Apple-inspired restraint, native typography, considered spacing, minimal surfaces, contextual actions, accessible controls, and reduced motion. The user delegated detailed product and visual decisions. Relay is the working product name.

## Product principles

- Keep the previous working version until the replacement is installed.
- Delete only resources whose ownership is established.
- Show real progress and actionable failures.
- Compile source automatically and remove build caches after each operation.

## Confirmed defaults

Follow system appearance. Use the existing GitHub CLI sign-in when available, with an optional token stored in the OS credential store. Continue this session with the installed design skill.

## Accessibility

Keyboard navigation, visible focus, labelled controls, live operation status, appropriate contrast, and reduced-motion support.
