---
name: Relay
description: A compact native utility for personal GitHub apps.
colors:
  canvas: "#fafaf9"
  surface: "#ffffff"
  inset: "#efefec"
  hover: "#e8e8e4"
  text: "#262625"
  secondary: "#646461"
  tertiary: "#6c6c68"
  line: "#e7e7e3"
  accent: "#1862da"
  accent-hover: "#0c51c3"
  accent-soft: "#e9f0fc"
  danger: "#bd313d"
  danger-soft: "#fff0f1"
  success: "#27805d"
  warning: "#8b6415"
  dark-canvas: "#1c1c1b"
  dark-surface: "#242423"
  dark-inset: "#2c2c2a"
  dark-hover: "#343432"
  dark-text: "#efefec"
  dark-secondary: "#b8b8b2"
  dark-tertiary: "#adada6"
  dark-line: "#383835"
  dark-accent: "#81b2ff"
  dark-accent-hover: "#a0c4ff"
  dark-accent-soft: "#293d5a"
  dark-danger: "#ff929b"
  dark-danger-soft: "#492b32"
  dark-success: "#7cceac"
  dark-warning: "#e2bc69"
typography:
  window-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "19px"
    fontWeight: 600
    letterSpacing: "-.025em"
    lineHeight: "1.18"
  empty-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "20px"
    fontWeight: 500
    letterSpacing: "-.02em"
  dialog-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "22px"
    fontWeight: 650
    letterSpacing: "-.025em"
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "1.45"
  label:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "12px"
    fontWeight: 600
  metadata:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "11px"
    fontWeight: 400
rounded:
  control: "7px"
  icon-control: "6px"
  monogram: "10px"
  group: "12px"
  dialog: "16px"
spacing:
  action-gap: "8px"
  identity-gap: "14px"
  row-gap: "20px"
  content-inset: "36px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.surface}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 13px"
  button-secondary:
    backgroundColor: "{colors.inset}"
    textColor: "{colors.text}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 13px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.secondary}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 13px"
  button-danger:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.surface}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 13px"
  field:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "8px 10px"
  storage-summary:
    textColor: "{colors.secondary}"
    padding: "12px 0 20px"
  app-row:
    padding: "21px 0"
  switch:
    width: "34px"
    height: "20px"
    rounded: "{rounded.group}"
---
# Design System: Relay

## Overview

**Creative North Star: "Quiet native utility"**

Relay is a quiet native utility: warm neutral surfaces, system typography, fine separators, and compact blue actions. App identity, current state, and the next action carry the hierarchy.

Light, dark, and system appearance use the same semantic roles and synchronize the native window theme with the resolved appearance. Neutral monograms and Lucide line icons keep the interface practical; no generated imagery is needed.

**Key Characteristics:**

- Warm neutrals with restrained blue actions.
- Compact rows and contextual controls.
- Native typography, visible focus, and reduced motion.

## Colors

### Primary

Blue marks primary actions, available updates, keyboard focus, and selected menu items. Accent hover and soft selection colors provide state changes without extra decoration.

### Neutral

Warm canvas, surface, inset, hover, and line roles separate content with small tonal steps. Text, secondary, and tertiary roles establish hierarchy. The `dark-` frontmatter entries mirror each role in dark appearance; system appearance is the default.

### Status

Danger pairs with a soft error background. Success and warning identify confirmed outcomes and recoverable issues; always retain a text label or icon.

**The Semantic appearance Rule.** Use semantic CSS variables so controls, text, and surfaces change together with appearance.

## Typography

Use the system font stack throughout. The compact window title has the strongest persistent emphasis; empty and dialog titles are larger only within their own context. Body text follows the root rhythm, action labels use semibold compact type, and repository metadata is secondary. App names inherit body sizing and bold emphasis. Details use a larger title (27px); section headings are usually compact (13–14px). Avoid display typography or promotional copy.

## Layout

The desktop utility has one centered content column capped at 860px, with a fixed header and independently scrolling content. Initial window size is 820×640; minimum is 640×500. macOS hides the duplicate native title while preserving its accessible window name. The shell fills the available webview height; content scrolls internally. Header insets are 36px at the top and sides, with 24px beneath; content uses 36px side and bottom insets.

The header carries the title, refresh, a Settings/Storage menu, and one Add app button. Settings and Storage replace the list and expose Back to apps. The app list uses fine top and row separators, a minimum row height of 94px, and 21px vertical padding. Updates sort first, then names; paused apps remain in the same list. The empty state points to the header action without repeating it. Full composition is specified in `.impeccable/surface-briefs/desktop.md`.

A defensive breakpoint at 600px reduces side insets to 20px, wraps row status below identity, and tightens action gaps. This is below the native minimum width, not a separate mobile product. Dialogs cap at 540px with viewport margins; app details occupy a right-side layer capped at 430px. Both allow content scrolling.

## Elevation & Depth

Ordinary content stays flat, separated by tone and fine lines. Menus, dialogs, and notices use the theme-aware shared layer shadow; details use a leftward shadow. A small switch-thumb shadow communicates its movable control. No blur or decorative elevation is used.

**The Layer depth Rule.** Keep ordinary content flat. Reserve broad shadows for temporary layers.

Layer entrances are short: menus (160ms), dialogs (220ms), and details (300ms). Button states transition over 150–180ms; pressed controls scale to .97. Reduced motion collapses animation and transition durations to .01ms and disables smooth scrolling.

## Shapes

Controls are gently rounded. Buttons and fields use the control radius; icon buttons are slightly tighter. App monograms and menus use 10px corners, grouped inset surfaces use 12px, and dialogs use 16px. Normal app monograms are neutral 44px squares; large details and small storage variants are 76px and 38px. Borders are fine separators, not card outlines.

## Components

- **Buttons:** Primary blue for Add, Install, and Update; neutral secondary for Open and routine actions; ghost for low emphasis; danger for explicit destructive confirmation. Minimum height is 33px, not a fixed height. Hover changes tone; disabled opacity is .48. Icon buttons are 30px square with accessible names.
- **Fields:** Surface fill, fine line border, visible labels, and muted placeholders. Repository entry is an inset grouped field. All interactive controls receive a 3px accent focus outline with a 3px offset.
- **App rows:** Neutral monogram, app name and repository, one current state, a main action, and an overflow menu. Identity opens details. Right-click and overflow share lifecycle actions; update rows retain a compact Open action.
- **Menus:** Surface fill, 10px corners, 5px outer padding, and accent-highlighted items. Settings and Storage live in the header menu; lifecycle actions remain contextual.
- **Settings and storage:** Flat separated rows. Managed versions, its amount, and refresh share one compact summary row. Cleanup heading and total appear only while loading, on error, or when items exist; the clear state says Nothing to clean up once. Development caches have a short explanation and Choose folder action. Switches use a 34×20px track and 16px thumb; amounts use tabular numerals.
- **Dialogs and details:** Focused temporary layers for installation, recovery, ownership, removal, and per-app history. Installation trust guidance is one useful sentence. Keep long failures and paths scrollable or wrapped. Details appear only after selection.
- **Operations and notices:** Real operation state appears in a bottom strip with progress and Cancel. Notices are dismissible; failures retain concise recovery guidance. Use status and alert semantics rather than color alone.

## Do's and Don'ts

### Do:

- **Do** keep action labels direct and show each current state once.
- **Do** preserve visible keyboard focus, labelled controls, live progress, cancellation, and reduced motion.
- **Do** keep long names, paths, and errors readable through wrapping, detail access, or scrolling.
- **Do** use the existing system type stack, neutral monograms, and Lucide line icons.

### Don't:

- **Don't** add slogans, redundant descriptions, permanent engineering caveats, or unsupported claims.
- **Don't** add a sidebar, search bar, global Activity view, category navigation, or repeated Add app controls to the desktop utility.
- **Don't** use color alone for status or make ordinary rows into floating cards.
