## sw_galaxy_map_cli

## [0.15.4] - 2026-04-08

### Added

* Added `db backup` command to create a physical copy of the SQLite database with a timestamped filename.
* Added `db export` command to export supported tables to CSV or JSON.

    * Requires `--table <name>` and one of `--csv` or `--json`.
    * Output files are generated automatically as `<table>_<timestamp>.<ext>`.
* Added optional `--output <dir>` parameter for both `db backup` and `db export`.

    * When provided, commands run non-interactively without prompting the user.

### Changed

* Improved export UX by requiring only a destination directory instead of a full file path.
* Standardized timestamp-based file naming across backup and export operations.
* Unified CLI behavior for interactive vs non-interactive execution.

### Fixed

* Fixed export path handling by preventing directory-as-file errors.
* Fixed incorrect handling of output paths with trailing separators on Windows.
* Improved argument validation for export format flags.

### TUI

* Blocked `db backup` and `db export` commands in TUI mode.
* Added contextual CLI guidance messages when these commands are invoked in TUI.

### Improved

* Added file size reporting after backup and export operations.

    * Displays human-readable sizes (KB, MB, GB).
* Improved feedback clarity by printing destination file path and generated file name.

### Refactored

* Simplified TUI command handling with a unified logging helper (`tui_log_only`).
* Centralized CLI-only command detection via `tui_only_cli_message`.
* Introduced shared helper for human-readable file size formatting.

### Notes

* Export is currently limited to a whitelist of supported tables for safety.
* Advanced filtering or partial export is not yet implemented.

---

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
