---
name: Relay
description: A native desktop library for personal GitHub applications.
colors:
  canvas: "#fafbfc"
  surface: "#ffffff"
  sidebar: "#edf0f5"
  sidebar-selected: "#dce5f4"
  inset: "#f0f2f6"
  hover: "#eceff4"
  text: "#242a36"
  secondary: "#5e6676"
  tertiary: "#626a79"
  line: "#e5e8ee"
  accent: "#1862da"
  accent-hover: "#0c51c3"
  accent-soft: "#e9f0fc"
  danger: "#bd313d"
  danger-soft: "#fff0f1"
  success: "#27805d"
  warning: "#8b6415"
  dark-canvas: "#1b1d22"
  dark-surface: "#24272e"
  dark-sidebar: "#202329"
  dark-sidebar-selected: "#303f58"
  dark-inset: "#2c3038"
  dark-hover: "#343943"
  dark-text: "#f0f2f6"
  dark-secondary: "#b3bac8"
  dark-tertiary: "#a4acbb"
  dark-line: "#353a44"
  dark-accent: "#81b2ff"
  dark-accent-hover: "#a0c4ff"
  dark-accent-soft: "#293d5a"
  dark-danger: "#ff929b"
  dark-danger-soft: "#492b32"
  dark-success: "#7cceac"
  dark-warning: "#e2bc69"
typography:
  headline:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "31px"
    fontWeight: 680
    lineHeight: 1.18
    letterSpacing: "-.035em"
  welcome-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "34px"
    fontWeight: 600
    lineHeight: 1.14
    letterSpacing: "-.035em"
  dialog-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "22px"
    fontWeight: 650
    lineHeight: 1.45
    letterSpacing: "-.025em"
  section-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "18px"
    fontWeight: 650
    lineHeight: 1.45
    letterSpacing: "-.02em"
  item-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "15px"
    fontWeight: 620
    lineHeight: 1.45
    letterSpacing: "-.015em"
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
  compact-body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1.45
  metadata:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.45
  small-metadata:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "10px"
    fontWeight: 400
    lineHeight: 1.45
rounded:
  control: "7px"
  navigation: "8px"
  menu: "10px"
  surface: "12px"
  app-icon: "13px"
  dialog: "16px"
spacing:
  tight: "4px"
  control-gap: "7px"
  small: "8px"
  related: "12px"
  group: "16px"
  section: "20px"
  inset: "26px"
  panel: "28px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.surface}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 13px"
    height: "33px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
  button-secondary:
    backgroundColor: "{colors.inset}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "6px 13px"
    height: "33px"
  button-secondary-hover:
    backgroundColor: "{colors.hover}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.secondary}"
    rounded: "{rounded.control}"
    padding: "6px 13px"
    height: "33px"
  button-danger:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.surface}"
    rounded: "{rounded.control}"
    padding: "6px 13px"
    height: "33px"
  navigation-selected:
    backgroundColor: "{colors.sidebar-selected}"
    textColor: "{colors.accent}"
    rounded: "{rounded.navigation}"
    padding: "0 12px"
    height: "39px"
  field:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "8px 10px"
  storage-summary:
    backgroundColor: "{colors.inset}"
    rounded: "{rounded.surface}"
    padding: "26px"
  app-row:
    padding: "18px 0"
  switch:
    backgroundColor: "{colors.hover}"
    rounded: "{rounded.surface}"
    width: "34px"
    height: "20px"
---
# Design System: Relay

## Overview

**Creative North Star: "Native desktop library"**

Relay uses the familiar structure of a native desktop library. System typography, cool neutral surfaces, blue actions, and separated app rows keep names and actions easy to scan. The sidebar has an opaque neutral fill that suggests native material without blur.

The interface follows system appearance by default and also supports explicit light and dark choices. Motion explains temporary layers and operation state. App identities use deterministic monograms and Lucide line icons. The geometric desktop icon is authored SVG, with generated desktop icon provenance in `src-tauri/icons/PROVENANCE.md`.

**Key Characteristics:**

- System typography and appearance.
- Separated rows with contextual actions.
- Short layer transitions with reduced-motion support.

## Colors

Cool whites and graphite neutrals support blue actions. The frontmatter records light values under their CSS names and dark overrides with the `dark-` prefix. Runtime components use the unprefixed CSS variable names in both themes.

### Primary

Action blue (`accent`) marks primary buttons, updates, selected navigation, focus, selection, and the caret. `accent-hover` strengthens the hover state; `accent-soft` supplies a quiet icon or selection background.

### Neutral

`canvas` is the main content background; `surface` is the field and floating-layer background. `sidebar` and `sidebar-selected` define navigation. `inset` and `hover` define grouped content and control states. `text`, `secondary`, and `tertiary` express the text hierarchy. `line` separates rows and sections.

Success green, warning ochre, and danger red are semantic status colors. Danger uses `danger-soft` for error feedback. Pair status color with text and an icon or other explicit state.

**The Semantic appearance Rule.** Use the semantic CSS variables so text, actions, and surfaces change together between light and dark.

## Typography

The system stack in the frontmatter is shared by all UI roles. This is a desktop application, so system headings are part of the confirmed native direction. There is no separate branded display face.

The headline role is the page title; compact desktop mode reduces it to 28px. The larger welcome title belongs to first run; it is not a universal page scale. Dialog and section titles step down before app names, body copy, controls, and metadata. App names use the item-title role. The body defaults to the root size; much of the operational copy uses compact-body. Explanatory paragraphs commonly raise line height to 1.7 or 1.8. Counts, versions, and storage figures use tabular numerals where aligned comparison matters.

**The Native type Rule.** Use the system font stack for application headings, controls, and copy. Keep monospaced text for actual code or paths that need it.

## Layout

The application fills the native window with a constrained grid row and shrinking main and content regions. The configured initial window is 1160 by 790px, with a supported minimum of 760 by 560px. The shell has a fixed sidebar and a flexible content column. The toolbar stays above independently scrolling content; an operation strip can sit below it.

The sidebar is 208px normally, 180px at widths up to 1000px, and 66px at widths up to 780px. The compact sidebar hides visible labels, add action, and footnotes while preserving labelled icon navigation. The toolbar is 70px high. Normal content padding is 39px 38px 65px; it becomes 32px 25px 50px at the intermediate breakpoint and 30px 22px in compact mode. Settings content stops at 800px wide.

Use tight spacing within a name and its metadata, then wider gaps between actions and groups. App rows have a 105px minimum height. Main content expands rather than centering in a narrow marketing container. Existing rules below 560px are defensive web layout code outside the supported desktop window range; they do not establish a mobile product.

## Elevation & Depth

Separators and tonal changes define persistent layout. Floating layers use soft shadows. The light layer shadow is `0 16px 60px #15223c24, 0 3px 12px #15223c0d`; its dark override is `0 16px 60px #0006, 0 3px 12px #0003`. The right panel uses `-12px 0 45px #0d1b3320`. Dialog overlays use `#111a2d36`; details overlays use `#111a2d20`. There is no decorative backdrop blur.

**The Layer depth Rule.** Keep the main library flat. Reserve broad shadows for menus, dialogs, notices, and the details panel.

## Shapes

Controls use modest rounded corners. Navigation is slightly rounder, menus have their own radius, and grouped storage content uses the shared surface radius. Dialogs use a larger radius. App monograms are square tiles with gently curved corners, with normal, large, and small sizes. Pills are limited to switch tracks and small indicators.

## Components

### Buttons

Primary actions use accent on surface-colored text; secondary actions use inset on text; ghost actions have a transparent background and secondary text. Destructive actions use danger. Shared controls have a minimum height of 33px, with the frontmatter padding and radius. The height token describes that minimum. Hover changes background; press scales enabled buttons to .97. Disabled controls use .48 opacity and the default cursor. Icon buttons are 30px squares with 6px corners.

All keyboard-focusable buttons, fields, selects, summaries, and menu items receive a 3px accent outline offset by 3px. Preserve the labelled accessible names when a control shows only an icon.

### Inputs / Fields

Fields use a surface fill, a 1px separator-colored border, and the shared control radius. Placeholder text uses tertiary and the caret uses accent. Search is a 31px inset container with an icon, borderless field, and keyboard hint. Repository entry groups its icon and field in an inset container.

Settings switches use a 34 by 20px track and a 16px circular surface-colored thumb. The checked thumb moves 14px and the track becomes accent. Track and thumb changes take 180ms. Checkboxes remain square native controls with accent coloring for confirmations.

### Navigation

Navigation rows are 39px high. Resting text and line icons use secondary. Hover uses hover fill and text color. Selected rows use sidebar-selected fill, accent, and weight 600. Compact navigation keeps accessible names and titles when labels disappear.

### Cards / Containers

The library uses separated rows. A storage summary groups an icon, amount, explanation, and protection note on an inset surface with the frontmatter radius and padding. Settings and history use lines between groups. Avoid adding shadows to these persistent content groups.

### App rows and identity

Rows align the monogram, app name and repository, version or update status, and immediate action. The context menu holds lifecycle actions. Normal monograms are 54px square with 25px initials; details use 76px tiles with 35px initials, and small variants use 38px tiles with 19px initials. Hue derives from the app name, with `hsl(var(--app-hue), 34%, 90%)` fill and `hsl(var(--app-hue), 42%, 30%)` lettering. These identity colors are independent of the theme palette.

### Temporary layers and motion

Context menus use a surface fill, shared layer shadow, 5px internal padding, and 10px outer corners. Highlighted items use accent with surface-colored text. Menu items are 12px with 8px 9px padding. Dialogs are at most 540px wide, with 26px padding, constrained height, and internal scrolling. App details enter from the right in a panel up to 430px wide, with 28px content padding.

Menus enter over 160ms with a 3px translation. Dialogs enter over 220ms with a small translation and scale. Details enter over 300ms with a 35px translation. The standard easing is `cubic-bezier(.22, 1, .36, 1)`; layers use `cubic-bezier(.2, .85, .25, 1.05)` for a slight spring. Loading and live operation indicators may pulse or rotate. Reduced-motion mode sets animation and transition durations to .01ms, limits animation to one iteration, and restores automatic scrolling.

## Do's and Don'ts

### Do:

- Do use semantic theme variables and preserve the system appearance default.
- Do preserve labelled keyboard controls, visible focus, live operation status, and reduced-motion support.
- Do use the existing Lucide stroke icons and deterministic app monograms.
- Do keep repository names, paths, and failures readable through wrapping, truncation with detail access, or scrolling.

### Don't:

- Don't add decorative blur, invented app imagery, or card grids to the native row system.
- Don't use color alone to communicate an error, update, or operation state.
- Don't replace contextual lifecycle actions with permanent controls on every row.

Evidence: `src/styles/tokens.css`, `src/styles/app.css`, `src/components/ui.tsx`, `Library.tsx`, `Sidebar.tsx`, `SettingsView.tsx`, and `src-tauri/tauri.conf.json`. Windows captures reviewed in `.impeccable/review/` include `final-library-light.png`, `final-settings-light.png`, `final-minimum-window.png`, and the system-dark empty state in `desktop.png`. macOS rendering has not been verified.

Not canonized: existing 9px hints and tertiary metadata are a small-text legibility limitation, not a reusable type role. No source defects were repaired in this documentation pass.
