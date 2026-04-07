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

## sw_galaxy_map_cli

### [0.15.3] - 2026-04-07

### ♻️ Refactor

- Completed the TUI refactor by moving the remaining interactive runtime out of the CLI module tree into `src/tui/`.
- Replaced the former monolithic TUI file with dedicated modules:
    - `tui/runtime.rs`
    - `tui/app.rs`
    - `tui/render.rs`
    - `tui/input.rs`
    - `tui/log.rs`
- Kept `tui/bridge.rs`, `tui/panels.rs`, and `tui/types.rs` as the shared TUI integration layer.
- Improved separation of concerns between:
    - TUI runtime bootstrap
    - TUI state management
    - rendering
    - input handling
    - log/typewriter orchestration
- Removed residual TUI-specific dependencies from the CLI core, further reducing the responsibility of `cli/*`.
- Preserved existing behavior while substantially improving maintainability and internal structure.

---

### [0.15.2] - 2026-04-04

### ♻️ Refactor

- Continued the CLI refactor after the `route` module split introduced in `0.15.1`.
- Reduced `cli/mod.rs` to a minimal entrypoint centered on `run()`.
- Extracted CLI orchestration and runtime responsibilities into dedicated modules:
    - `dispatch.rs`
    - `db_runtime.rs`
    - `shell.rs`
    - `reports.rs`
- Moved TUI-specific structures and rendering logic out of the CLI core into the dedicated `tui` module:
    - `tui/mod.rs`
    - `tui/types.rs`
    - `tui/panels.rs`
    - `tui/bridge.rs`
- Improved separation of concerns between:
    - CLI argument dispatch
    - database bootstrapping/runtime helpers
    - reporting/output
    - TUI state/types
    - TUI rendering/bridging
- Preserved existing behavior while substantially improving maintainability and code organization.

---

### [0.15.1] - 2026-04-03

### ♻️ Refactor

- Split the monolithic `route` command module into focused submodules:
    - `compute.rs`
    - `show.rs`
    - `list.rs`
    - `cleanup.rs`
    - `explain.rs`
    - `types.rs`
- Reduced `route/mod.rs` to a lightweight dispatcher/orchestrator.
- Improved maintainability and separation of concerns without changing CLI behavior.

### 🐛 Fixed

- Fixed fuzzy search ignoring structured filters (`--region`, `--sector`, `--grid`, `--status`, `--canon`, `--legends`).
- Fixed fuzzy search always forcing active-only filtering, preventing queries like `--status deleted` from returning
  results.
- Fixed incorrect handling of fuzzy results in CLI (`search.rs`) where `FuzzyHit` and `PlanetSearchRow` pipelines were
  mixed.
- Fixed type mismatch and runtime inconsistencies by introducing a proper fuzzy → structured search pipeline (
  `fuzzy_search_filtered`).
- Fixed incorrect result ordering after SQL hydration by restoring original fuzzy ranking.
- Fixed compilation errors caused by outdated `fuzzy_search` call sites (missing `status` argument).

---

### [0.15.0] - 2026-04-03

### ✨ Added

- **Fuzzy search** with Levenshtein distance matching:
    - New `--fuzzy` flag on the `search` command for explicit fuzzy mode
    - Automatic "Did you mean?" suggestions when exact search returns 0 results
    - Full integration in both CLI and TUI (fuzzy results are selectable in TUI)
- **Enriched `route explain`** output:
    - Structured ETA breakdown
    - Waypoint-by-waypoint table with segment distance, cumulative distance, and label
    - Detour summary with total overhead, average score, average severity, worst detour, exhausted tries count
    - All sections also rendered in the TUI log panel
- **CSV polyline export** for routes:
    - New `--csv <path>` flag on `route explain`
    - Columns: `seq`, `x`, `y`, `segment_parsec`, `cumulative_parsec`, `label`
    - CSV-safe label escaping

### 🔄 Changed

- `search` command has new `--fuzzy` flag.
- `RouteExplainArgs` now includes `--csv`.
- `route explain` text output reorganized with ETA breakdown, waypoint table, and detour summary.
- TUI `build_route_show_output` enriched with segment distances, ETA breakdown, and detour summary.
- TUI search dispatch now handles explicit `--fuzzy` mode with selectable results.

---

### [0.14.0] - 2026-04-02

### ✨ Added

- **Combined search filters** on the `search` command:
    - `--region`
    - `--sector`
    - `--grid`
    - `--status`
    - `--canon`
    - `--legends`
- Added `db sync` command integrating the sync crate into the main CLI.
- Exposed `sw_galaxy_map_sync` functionality through the main CLI workflow.

### 🔄 Changed

- `search` command query is now optional when filters are provided.
- Validation flow updated to work with `SearchFilter`.
- Planet detail panels (TUI) now display Status.

### 🐛 Fixed

- Fixed broken revival logic surfaced through the CLI update flow.
- Fixed SQL alias error in `mark_deleted_missing()`.
- Fixed dry-run helper behavior aligned to `status`.

---

### [0.13.0] - 2026-04-01

### ✨ Added

- Added `db rebuild-search` command.
- Added `route explain` support for:
    - `--class`
    - `--region-blend`
    - `--sublight-kmps`
- Added route ETA display and explain integration.
- Added `planets.status` awareness across CLI-visible search behavior.

### 🔄 Changed

- `search` command simplified to `search <query> [--limit N]`.
- Removed legacy hybrid/official search paths from CLI.
- TUI search/result flow simplified.

### 🐛 Fixed

- Search queries now correctly filter out soft-deleted/skipped/invalid planets via `status`.

---

### [0.12.0] - 2026-03-27

### ✨ Added

- Introduced **Navigation panel** in TUI.
- Added **typewriter effect** for log output.
- Added support for **route list selection → route show integration**.

### 🔄 Changed

- Refactored TUI layout and panel structure.
- Improved visual alignment and navigation panel rendering.
- Improved `near` command UX and data placement.

### 🐛 Fixed

- Fixed log auto-scroll and typewriter/log interaction issues.
- Fixed incorrect panel updates after route selection.
- Fixed focus inconsistencies after `near` selection.

---

### [0.11.0] - 2026-03-26

### ✨ Added

- Interactive TUI (ratatui-based) for CLI usage.
- Scrollable panels, command history, selection flows, and context-aware help.

### 🔄 Changed

- Refactored `near` command to use positional planet argument and `--range`.
- Unified rendering of planet data across CLI and TUI.

### 🧹 Removed

- Deprecated `--planet` flag in `near`.
- Removed unused `build_near_hit_panel` helper.

### 🐛 Fixed

- Consistent rendering of `near` results when only one match is found.
- Various TUI UX issues.

---

### [0.9.10] - 2026-03-21

### Added

- Added `unknown near <planet> --range <parsecs>` command.

### Improved

- Improved unknown/known planet interoperability for nearby inspection.

### Fixed

- Fixed result collection type mismatch in unknown-near query handling.

---

### [0.9.9] - 2026-03-21

### ✨ Added

- Added X and Y coordinates to `search` command output.
- Improved CLI table formatting with dynamic column widths for coordinates.

### 🐛 Fixed

- Fixed `Invalid column index` error in `search`.
- Fixed missing waypoint/route-related schema issues affecting CLI commands.
- Improved robustness with incomplete or malformed records.

---

### [0.9.7] - 2026-03-20

### 🐛 Fixed

- Fixed crash in `unknown list` when encountering records with NULL `fid`.
- Fixed formatting issues when printing optional coordinates.
- Improved robustness of unknown planet handling from the CLI.

---

### [0.9.6] - 2026-03-20

### Added

- Added pagination support to `unknown list` with `--page` and `--page-size`.

### Changed

- `unknown list` now displays both internal unknown `ID` and source `FID`.
- `unknown search <id> --near <parsecs>` now resolves from internal unknown record ID.

---

### [0.9.5] - 2026-03-20

### Added

- Added `unknown list`.
- Added `unknown search <fid> --near <parsecs>` (later superseded by internal IDs).

---

### [0.9.1] - 2026-03-19

### Fixed

- Made the GUI/CLI integration more robust when the CLI sibling executable is not available.
- Restored the historical CLI binary name `sw_galaxy_map`.
- Added CLI library entry point for GUI help integration.

---

### [0.9.0] - Unreleased

### Changed

- Project reorganized as a Cargo workspace.
- `cargo run -p sw_galaxy_map_cli` now always starts only the CLI.
- Removed the legacy startup discriminant used to switch from CLI to GUI automatically.

---

### [0.8.2] - 2026-02-06

### Added

- Optional sublight travel time estimation in CLI output.
- `waypoint prune --include-linked`.

### Changed

- Running `sw_galaxy_map` with no arguments now launched the GUI.
- CLI had to be explicitly requested with `--cli`.

### Fixed

- Clippy/lint cleanups under `-D warnings`.

---

### [0.8.1] - 2026-02-10

### Added

- `route compute` now accepts two or more planets to compute multi-leg trips in one command.

---

### [0.8.0] - 2026-01-28

### ⚠️ Breaking change

- Running `sw_galaxy_map` with no arguments now entered Interactive CLI mode.
- `--gui` explicitly starts the GUI.

### Added

- Interactive CLI mode.
- `route explain` options: `--class`, `--region-blend`.
- ETA summary in `route show`.
- `waypoint links <id>` shows associated routes.
- `waypoint prune`.

### Changed

- Improved table rendering for `route list`.
- Improved `waypoint list`.

### Fixed

- Region parsing/selection issues in `route explain`.
- Various CLI plumbing issues.

---

### [0.7.5] - 2026-01-26

### ✨ New features

- Added hyperspace ETA estimation for routes.
- `route show` now displays a synthetic ETA.
- `route explain` supports ETA customization.

### 🛠 Improvements

- Refactored galactic region extraction into a shared helper.
- Clear separation between routing cost and navigational ETA estimation.

---

### [0.7.2] - 2026-01-26

### ✨ New Features

- Introduced a console-style GUI command bridge using the CLI.
- Added integrated Help system backed by real CLI help commands.
- Added status/feedback improvements relevant to CLI/GUI interaction.

### 🛠 Improvements

- Unified validation logic across CLI and GUI.
- Extended `route list` with `--wp <count>`.
- Improved tabular formatting and alignment.
- Refactored database migration handling from CLI entrypoints.

### 🧹 Fixes

- Resolved release build issues and command/help integration issues.

---

### [0.7.1] - 2026-01-24

### Added

- GUI status bar improvements and validation support connected to CLI behavior.

### Changed

- Minor UX polish and clearer status messaging.

---

### [0.7.0] - 2026-01-22

### Changed

- Application startup logic now selects CLI or GUI mode depending on provided arguments.

### Notes

- CLI functionality remained backward-compatible.

---

### [0.6.2] - 2026-01-20

### ✨ New Features

- Added `route list` command with filters, sorting, and JSON export.

### 🎨 UX Improvements

- Colorized `route list` output.
- Improved file/stdout export behavior.

### 🛠 Internal / Refactoring

- Introduced `RouteListOptions`.
- Refactored dynamic SQL generation and file output handling.

### 🐛 Bug Fixes

- Fixed SQL parameter count mismatch and JSON export path handling.

---

### [0.6.1] - 2026-01-20

### ✨ New Commands

- Added `route clear`.
- Added interactive confirmation and `--yes`.
- Added `route prune`.

### 🎨 UX Improvements

- Introduced colorized output for destructive and cleanup commands.

### 🛠 Internal / Refactoring

- Reused centralized confirmation logic.

---

### [0.6.0] - 2026-01-20

### ✨ New Features

- Introduced `route explain <id>`.
- Added detour telemetry.
- Implemented JSON export for route explanations.
- Unified CLI color policy across `route show` and `route explain`.

### 🧠 Improvements

- Improved detour scoring diagnostics.
- Hardened route explanation against legacy routes.

### 🛠 Internal / Refactoring

- Added shared `cli::color` helpers.
- Improved structure for future machine-consumable output.

---

### [0.5.3] - 2026-01-19

### ✨ Routing & Persistence

- Stabilized routing engine from the CLI point of view.
- Persisted detour waypoints and route data.
- Improved `route show` output with planet names and semantic labels.

### 🧪 Tests

- Added routing integration tests.

### 📚 Documentation

- Documented `--safety`, detour waypoint selection, and routing DB design.

---

### [0.5.2] - 2026-01-19

### ✨ Routing & Persistence

- Introduced full persistence for computed routes.
- Added `route compute`, `route last`, `route show`.

### 🧪 Tests

- Added routing integration tests.

### 🗄️ Database

- Added schema support for routes and detours.

### 🧹 Internal

- Improved separation between routing logic, persistence, and CLI.

---

### [0.5.1] - 2026-01-19

### 🚀 Routing Engine

- Added first working `route <FROM> <TO>` command with obstacle-aware routing.
- Added tunable routing parameters via CLI flags.

### 🔍 Debug & Observability

- Added detailed routing debug output.

### 🧱 Internal Refactor

- Separated routing logic into dedicated module.

---

### [0.5.0] - 2026-01-16

### Added

- Introduced waypoint catalog and CLI waypoint command group:
    - `waypoint add`
    - `waypoint list`
    - `waypoint show`
    - `waypoint delete`

### Changed

- Moved DB-related sources under `src/db/`.
- Improved schema migration workflow.

### Fixed

- Fixed migration correctness and negative coordinate parsing.

---

### [0.4.1] - 2026-01-16

### Changed

- Refactored the database layer from the CLI point of view.
- Reorganized the `db` module structure.

### Added

- Added planet fandom URL in `info` command output.

### Fixed

- Fixed invalid column name issues and query/result handling problems.

---

### [0.4.0] - 2026-01-16

### Added

- New `db update` command with incremental update logic.
- `--prune`, `--dry-run`, `--stats` flags.
- Extended `db status`.

### Improved

- More robust handling of invalid rows and clearer CLI output.

---

### [0.3.0] - 2026-01-15

### Added

- Unified colored console messaging system.
- Centralized UI output handling.

### Changed

- Refactored CLI output to use the new UI messaging layer.

### Fixed

- Resolved type mismatch issues related to colored output.

---

### [0.2.0] - 2026-01-15

### Added

- Automatic local database initialization on first use.
- `db status` command.
- FTS5-backed search when available.
- Alias-based planet lookup.

### Improved

- Database provisioning to local application data directory.
- Better search relevance and performance.
- Improved `db init` UX.

---

### [0.1.0] - 2026-01-15

### Added

- Initial release of the `sw_galaxy_map` CLI application.
- SQLite-backed local database for offline planet queries.
- Search, info, and nearby planet lookup commands.

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
