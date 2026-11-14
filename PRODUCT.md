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

Paste a GitHub URL, install a verified application package, open it, update it safely, and remove resources with known ownership. Keep build and version clutter under control.

## Operating context

Prefer GitHub Releases and artifacts from successful workflows on main. Detect common project stacks without requiring existing projects to be rewritten. Local builds require explicit trust and run in disposable workspaces. They are not an operating-system security sandbox.

## Capabilities and constraints

Stable repository identities, stable launch shortcuts, recoverable version selection, private repository access, release channels, history, repair, stop tracking, and scoped cache cleanup. Never infer ownership of unrelated files or silently terminate running apps. Do not bypass platform signing checks. No administrator privileges by default.

Arbitrary repositories cannot always be packaged automatically. Missing or ambiguous packages need a clear explanation and an explicit selection. Applications requiring system installers, privileged services, or drivers need their native installer and cannot receive the same transaction guarantee as portable packages.

## Brand commitments

Apple-inspired restraint, native typography, considered spacing, minimal surfaces, contextual actions, accessible controls, and reduced motion. The user delegated detailed product and visual decisions. Relay is the working product name.

## Product principles

- Keep the previous working version until the replacement is installed.
- Delete only resources whose ownership is established.
- Show real progress and actionable failures.
- Prefer successful remote builds over local toolchains.

## Confirmed defaults

Follow system appearance. Use the existing GitHub CLI sign-in when available, with an optional token stored in the OS credential store. Continue this session with the installed design skill.

## Accessibility

Keyboard navigation, visible focus, labelled controls, live operation status, appropriate contrast, and reduced-motion support.
