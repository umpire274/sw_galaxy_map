# Changelog

All notable changes to this repository will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and each crate adheres to [Semantic Versioning](https://semver.org/).

---

## Versioning policy

Starting after `0.15.1`, workspace crates use **independent versioning**.

- `0.15.1` is the last release where all crates shared the same aligned version.
- From `0.15.2` onward, each crate is versioned and released independently.
- This changelog is therefore organized **per crate**, not as a single global project version.

---

## sw_galaxy_map_gui

### [0.15.1] - 2026-04-03

(no changes)

---

### [0.15.0] - 2026-04-03

### ✨ Added

- Full TUI-related rendering integration for fuzzy results, route explain enrichment, and galaxy stats support surfaced
  to GUI-adjacent workflows where applicable.

---

### [0.14.0] - 2026-04-02

### 🔄 Changed

- GUI validation and search integration updated to use `SearchFilter`.
- GUI integration updated to use the sync library flow.

---

### [0.13.0] - 2026-04-01

(no direct GUI-specific changes separated in the original changelog)

---

### [0.12.0] - 2026-03-27

### ✨ Added

- Introduced Navigation panel in the TUI/GUI-adjacent interaction model.
- Added support for route list selection → route show integration in panels.

### 🔄 Changed

- Improved panel structure, alignment, and focus behavior.

### 🐛 Fixed

- Fixed panel update and scrolling inconsistencies.

---

### [0.9.1] - 2026-03-19

### Fixed

- Made the GUI more robust when the CLI sibling executable is not available.
- Vendored the GUI icon inside the `sw_galaxy_map_gui` crate.
- Improved GUI command/help fallback behavior after the workspace split.

### Changed

- Updated packaging layout to keep GUI assets self-contained.

---

### [0.9.0] - Unreleased

### Changed

- Project reorganized as a Cargo workspace with `sw_galaxy_map_gui` extracted as a dedicated crate.

---

### [0.8.2] - 2026-02-06

### Changed

- Running `sw_galaxy_map` with no arguments launched the GUI by default.

---

### [0.8.0] - 2026-01-28

### Added

- `--gui` flag to explicitly start the GUI.

### ⚠️ Breaking change

- No-args startup entered Interactive CLI mode instead of GUI.

---

### [0.7.2] - 2026-01-26

### ✨ New Features

- Introduced a console-style GUI.
- Added integrated Help window.
- Added status bar, boot sequence, DB indicator, and command output rendering.

### 🛠 Improvements

- Improved selection behavior in GUI text areas.
- Improved status and feedback UX.

### 🧹 Fixes

- Fixed text selection issues and release-mode GUI integration issues.

---

### [0.7.1] - 2026-01-24

### Added

- Embedded application icons for Windows and Linux.
- GUI status bar improvements and tooltips.

---

### [0.7.0] - 2026-01-22

### Added

- Introduced graphical Navicomputer UI.
- FROM/TO input fields, Compute/Clear actions, output area, JSON export, status bar.

---

## sw_galaxy_map_sync

### [0.15.1] - 2026-04-03

(no changes)

---

### [0.15.0] - 2026-04-03

(no direct sync-specific changes separated in the original changelog)

---

### [0.14.0] - 2026-04-02

### ✨ Added

- Exposed `sw_galaxy_map_sync` as a library crate (`lib + bin`).
- Public API:
    - `run_sync()`
    - `SyncOptions`
    - `SyncResult`
    - `resolve_csv_path()`

### 🔄 Changed

- `sw_galaxy_map_sync` binary now delegates to `run_sync()` from the library.

---

### [0.13.0] - 2026-04-01

### ✨ Added

- Introduced **`sw_galaxy_map_sync`** crate for synchronizing the official Lucasfilm catalog into the `planets` table.
- Reads CSV, matches against existing DB records, updates `status`, generates XLSX sync report.
- Includes progress bar and dry-run mode.
- Unit tests for CSV matching strategies.

### 🧠 Internal

- Added sync-specific dependencies and packaging structure.
