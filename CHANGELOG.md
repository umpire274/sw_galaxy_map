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

## sw_galaxy_map_core

### [0.15.1] - 2026-04-03

(no changes)

---

### [0.15.0] - 2026-04-03

### ✨ Added

- New `GalaxyStats` struct in `sw_galaxy_map_core::model`.
- New `galaxy_stats()` query function with aggregate SQL queries.
- New `FuzzyHit` struct and `fuzzy` module in `sw_galaxy_map_core::utils`.

### 🔄 Changed

- `SearchFilter` struct now includes `fuzzy: bool`.

### ♻️ Refactor

- Split `db::queries` into domain modules (`planets`, `routes`, `waypoints`, `stats`, etc.).
- Introduced `row_mappers.rs` for centralized row → model mapping.
- Removed monolithic `queries/mod.rs` logic.
- Cleaned unused constants and imports.
- Preserved public API (no breaking changes).

---

### [0.14.0] - 2026-04-02

### ✨ Added

- New `SearchFilter` struct in `sw_galaxy_map_core::model`.
- New `search_planets_filtered()` query function with dynamic SQL construction.

### 🔄 Changed

- `PlanetSearchRow` now includes `status: Option<String>`.

### 🧠 Internal

- `validate_search()` refactored to accept a single `&SearchFilter`.
- Revival logic and update helpers aligned to `status`.

---

### [0.13.0] - 2026-04-01

### ✨ Added

- Added `db rebuild-search` support at the data layer.
- Introduced **hyperspace ETA estimation engine** (`routing::eta`, `routing::hyperspace`).
- Added **sublight travel time estimation** (`routing::sublight`).
- Added `planets.status` field support across the data layer.
- Added `seed_planets_official()` provisioning function.

### 🧠 Internal

- New `hyperspace`, `sublight`, and `eta` modules.
- New `rebuild_search_indexes()` public entry point.
- Routing module reorganized.

### 🐛 Fixed

- Search queries now correctly filter out soft-deleted/skipped/invalid planets via `status`.

---

### [0.12.0] - 2026-03-27

(no direct core-specific changes separated in the original changelog)

---

### [0.11.0] - 2026-03-26

(no direct core-specific changes separated in the original changelog)

---

### [0.9.10] - 2026-03-21

### Added

- Added core query helpers for nearby unknown planets.

---

### [0.9.9] - 2026-03-21

### 🐛 Fixed

- Fixed schema/runtime mismatches affecting route and unknown-planet support.
- Improved alignment between models and DB schema.

### 🧩 Internal

- Reduced type complexity in provisioning.
- Improved consistency between provisioning, migrations, and runtime schema.

---

### [0.9.7] - 2026-03-20

### 🐛 Fixed

- Updated `UnknownPlanet.fid` to `Option<i64>`.
- Ensured consistent handling of nullable fields across the DB layer.

---

### [0.9.6] - 2026-03-20

### Added

- Expanded `planets_unknown` schema for staging/edit workflows.
- Added workflow fields (`reviewed`, `promoted`, `notes`) and normalized storage.

---

### [0.9.5] - 2026-03-20

### Added

- Added core query helpers for listing/searching unknown planets.

---

### [0.9.1] - 2026-03-19

(no direct core-specific changes separated in the original changelog)

---

### [0.9.0] - Unreleased

### Changed

- Project reorganized as a Cargo workspace with `sw_galaxy_map_core` extracted as a dedicated crate.

---

### [0.5.3] - 2026-01-19

### ✨ Routing & Persistence

- Persistence of detour waypoints as `computed`.
- Deterministic fingerprint-based deduplication.
- Automatic waypoint → obstacle planet linking with role `avoid`.
- Full persistence of routes, route polyline, and detour decision details.

### 🧪 Tests

- Added routing integration tests.
- Shared helper `assert_collision_free`.

---

### [0.5.2] - 2026-01-19

### ✨ Routing & Persistence

- Introduced route persistence support and route-related schema up to v8.

### 🗄️ Database

- Added `routes`, `route_waypoints`, `route_detours`.

### 🧹 Internal

- Improved separation between routing logic and persistence.

---

### [0.5.1] - 2026-01-19

### 🚀 Routing Engine

- Implemented first working routing engine between two planets.
- Added obstacle-aware geometry and scoring model.

### 🧠 Collision Handling

- Unified collision handling and validation.

---

### [0.5.0] - 2026-01-16

### Added

- Introduced waypoint catalog and waypoint DB queries.

### Changed

- Moved DB-related sources under `src/db/`.

---

### [0.4.1] - 2026-01-16

### Changed

- Consolidated planet-related queries into `db/queries.rs`.
- Standardized column-name based row mapping.

### Added

- Added `Planet::info_planet_url()`.

### Fixed

- Fixed SQL alias drift and query closure result mismatches.

---

### [0.4.0] - 2026-01-16

### Added

- Incremental `db update` data-layer support with hashing and soft delete.

### Internal

- Shared normalization and hash computation reused across init/update.

---

### [0.2.0] - 2026-01-15

### Added

- Automatic local database initialization support.
- FTS/search data layer.
- Alias-based lookup support.

---

### [0.1.0] - 2026-01-15

### Added

- Initial database-backed planet/domain logic.

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
