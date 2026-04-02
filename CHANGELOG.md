# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.14.0] - 2026-04-02

### ✨ Added

- **Combined search filters** on the `search` command
    - `--region` (partial match, case-insensitive)
    - `--sector` (partial match, case-insensitive)
    - `--grid` (exact, case-insensitive)
    - `--status` (exact: active, inserted, modified, skipped, deleted, invalid)
    - `--canon` (show only Canon planets)
    - `--legends` (show only Legends planets)
    - Text query is now optional: at least one filter or a query is required
    - All active filters are combined with AND
    - Output table now includes a Status column
    - Result count and filter description shown at the bottom
- Added `db sync` command integrating the sync crate into the main CLI
    - Reads a CSV, matches against existing records, updates status fields
    - Automatically rebuilds `planet_search` and FTS indexes after sync
    - Supports `--csv`, `--table`, `--delimiter`, `--dry-run`, `--mark-deleted`, `--report`
    - Single command replaces the manual workflow of running `sw_galaxy_map_sync` + `db rebuild-search`
- New `SearchFilter` struct in `sw_galaxy_map_core::model` for type-safe filter passing
- New `search_planets_filtered()` query function with dynamic SQL construction
- Exposed `sw_galaxy_map_sync` as a library crate (`lib + bin`)
    - Public API: `run_sync()`, `SyncOptions`, `SyncResult`, `resolve_csv_path()`

### 🔄 Changed

- `search` command query is now `Option<String>` (optional when filters are provided)
- `validate_search()` refactored to accept a single `&SearchFilter` (Clippy-clean, no more than 7 args)
- `PlanetSearchRow` now includes `status: Option<String>` field
- Planet detail panels (TUI) now display Status
- `sw_galaxy_map_sync` crate restructured as lib + bin (Cargo.toml `[lib]` section added)
- `sw_galaxy_map_sync` binary now delegates to `run_sync()` from the library

### 🐛 Fixed

- **Fixed broken revival logic in `db update`**: the update flow was reading the legacy `deleted` INTEGER column for
  revival decisions while `mark_deleted_missing()` only wrote to `status`. A planet marked as deleted via status was
  never revived because `deleted` stayed at 0. Now the entire `db update` flow uses `status` consistently.
- **Fixed SQL error in `mark_deleted_missing()`**: the query used table alias `p.status` but the UPDATE had no alias on
  the `planets` table. Added `IS NULL` handling for planets with no status set.
- **Fixed dry-run helpers** (`select_missing_active_planets`, `count_missing_active_planets`): were filtering on legacy
  `deleted = 0` instead of `status`. Aligned to `status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid')`.

### 🧠 Internal

- `db_get_hash_and_deleted()` renamed to `db_get_hash_and_status()`, returns `Option<String>` instead of `i64`
- Revival logic checks `matches!(old_status.as_deref(), Some("deleted"))` instead of `old_deleted == 1`
- All `db_update.rs` queries now use `status` field consistently; legacy `deleted` column is no longer read
- GUI `validate_search` call updated to use `SearchFilter` with `..Default::default()`

---

## [0.13.0] - 2026-04-01

### ✨ Added

- Added `db rebuild-search` command to rebuild `planet_search` and FTS indexes from current `planets` data
    - Standalone entry point backed by `rebuild_search_indexes()` (transaction-safe)
- Introduced **`sw_galaxy_map_sync`** crate for synchronizing the official Lucasfilm catalog into the `planets` table
    - Reads the official CSV (~7000 systems from "Star Systems of the Galaxy", Lucasfilm 2023 PDF), normalizes and
      compares against existing DB records
    - Multi-strategy matching: exact name, Roman suffix stripping, sector/region/grid triple
    - Sets per-record `status` field: `active`, `inserted`, `modified`, `skipped`, `deleted`
    - Generates an XLSX sync report with per-row outcome breakdown
    - Includes progress bar (indicatif) and dry-run mode
    - Unit tests for CSV matching strategies
- Added **hyperspace ETA estimation engine** (`routing::eta`, `routing::hyperspace`)
    - Region-aware compression factor model with 10 galactic regions (Deep Core → Unknown Regions)
    - Detour penalty system: geometric, count-based, and severity-based multipliers
    - Configurable region blend policies: `avg`, `conservative`, `weighted(f64)`
    - Hyperdrive class multiplier support
    - `RouteEtaEstimate` struct with full breakdown of all contributing factors
    - Integrated into `route explain` with `--class` and `--region-blend` flags
- Added **sublight travel time estimation** (`routing::sublight`)
    - Parsec-to-km conversion using IAU constant
    - Configurable speed parameter via `--sublight-kmps` on `route explain`
- Added `planets.status` field support across the data layer
    - `search` queries now exclude planets with `status IN ('deleted', 'skipped', 'invalid')`
- Added `seed_planets_official()` provisioning function with CSV-to-DB import and auto-normalization

### 🔄 Changed

- `search` command simplified to `search <query> [--limit N]` (positional, non-optional query)
    - Removed `--official`, `--sector`, `--region`, `--grid` flags (official data now synced directly into `planets`)
    - Removed `run_official()` and `run_any()` hybrid search functions
    - Removed `SearchRow` enum from `model.rs`
    - Removed `search_any()` from `queries.rs`
- `validate_search()` simplified to `(query: &str, limit: i64)`
- TUI `SelectionMode` reduced from 5 to 4 variants (removed `OfficialSearch`)
- TUI `TuiCommandOutput` no longer carries `official_search_results`
- TUI result dispatch simplified from 4-tuple match to 3-tuple match
- Bumped workspace version to `0.13.0`
- Added `csv`, `rust_xlsxwriter`, and `indicatif` to workspace dependencies (for sync crate)

### 🧠 Internal

- New `hyperspace` module with `GalacticRegion` enum, region parsing, compression factor model, and detour penalty
  system
- New `sublight` module with parsec↔km conversion
- New `eta` module orchestrating all ETA components into a single `estimate_route_eta()` call
- `provision.rs`: added `rebuild_search_indexes()` public entry point
- Routing module reorganized: `eta.rs`, `hyperspace.rs`, `sublight.rs` alongside existing `router.rs`, `collision.rs`,
  `geometry.rs`

### 🐛 Fixed

- Search queries now correctly filter out soft-deleted/skipped/invalid planets via `status` column

---

## [0.12.0] - 2026-03-27

### ✨ Added

- Introduced **Navigation panel** in TUI (between Planet1 and Planet2)
    - Displays route/near relationship data (Length, ETA, Distance, Detours)
    - Adaptive content via `NavigationPanelKind` (Route / Near / Empty)
- Added **typewriter effect** for log output
    - Configurable speed and chunk size
    - Automatically disabled for long outputs
- Added support for **route list selection → route show integration**
    - Selecting a route now renders full route details in panels

### 🔄 Changed

- Refactored TUI layout:
    - Right side now split into:
        - Planet1
        - Navigation
        - Planet2
    - Navigation panel height increased for better readability
- Improved visual alignment in all right panels (Planet + Navigation)
    - Consistent key/value formatting via helper
- Improved `near` command UX:
    - Distance moved from Planet2 panel to Navigation panel
    - Planet panels now contain only planet-specific data
- Improved route rendering:
    - Cleaner header formatting
    - Improved Detours and Waypoints layout for readability
- Unified panel-building logic:
    - Introduced `build_navigation_panel(...)`
    - Cleaner separation between data and rendering

### 🧠 Internal

- Introduced `NavigationPanelKind` enum for adaptive rendering
- Refactored TUI output flow to include navigation panel data
- Improved state handling for selection modes (Search / Near / RouteList)
- Fixed panel focus logic after layout changes
- Fixed scrolling logic for new panel structure

### 🐛 Fixed

- Fixed log auto-scroll when content exceeds viewport
- Fixed typewriter/log interaction issues (flush on command)
- Fixed incorrect panel updates after route selection
- Fixed focus inconsistencies after `near` selection
- Fixed a TUI log scrolling bug where the active typewriter line could remain off-screen when the log already exceeded
  the viewport.

---

## [0.11.0] - 2026-03-26

### ✨ Added

- Interactive TUI (ratatui-based) for CLI usage
- Scrollable panels:
    - Log panel (left)
    - Planet 1 and Planet 2 panels (right)
    - Command input panel with history
- Command history navigation (↑ / ↓)
- System commands:
    - `:q`, `:quit`, `:exit`, `:x` to exit
    - `:help` for system help
- Interactive selection for:
    - `search` results
    - `near` results
- Automatic panel focus:
    - `search` → Planet 1
    - `near` selection → Planet 2
- Context-aware help line in TUI

### 🔄 Changed

- Refactored `near` command:
    - Planet is now a positional argument (`near coruscant -r 50`)
    - `--r` replaced with `--range` (alias `-r`)
- Refactored `unknown near` command for consistency with `near`
- Unified rendering of planet data across CLI and TUI
- Improved validation logic for `near`

### 🧹 Removed

- Deprecated `--planet` flag in `near`
- Removed unused `build_near_hit_panel` helper

### 🐛 Fixed

- Consistent rendering of `near` results when only one match is found
- Various minor TUI UX issues (cursor blink, scroll behavior, duplicated input)

---

## [0.9.10] - 2026-03-21

### Added

- Added `unknown near <planet> --range <parsecs>` command to find unknown planets near a known planet
- Implemented proximity search from `planets.X` / `planets.Y` to `planets_unknown`
- Added distance-sorted output for nearby unknown planets
- Added optional result limiting for nearby unknown planet search

### Improved

- Excluded unknown records without coordinates from proximity search
- Reused robust squared-distance SQL filtering for spatial lookup
- Improved interoperability between known-planet search and unknown-planet inspection workflows

### Fixed

- Fixed result collection type mismatch in unknown-near query handling

---

## [0.9.9] - 2026-03-21

### ✨ Added

- Added X and Y coordinates to `search` command output (tabular view)
- Improved CLI table formatting with dynamic column widths for coordinates

### 🐛 Fixed

- Fixed `Invalid column index` error in `search` due to mismatched SQL mapping
- Fixed missing `waypoints.fingerprint` column causing `route compute` failure
- Fixed missing route-related tables (`routes`, `route_waypoints`, `route_detours`) in `db init`
- Fixed initial database provisioning not populating `planets_unknown`
- Fixed mismatch between fresh schema (`db init`) and migrated schema

### 🔄 Improved

- Refactored `db_update` to preserve `planets_unknown` internal IDs (no full rebuild)
- Improved handling of unknown planets with missing data (`fid`, `x`, `y`)
- Improved robustness of CLI commands with incomplete or malformed records

### 🧩 Internal

- Reduced type complexity in `provision.rs` using dedicated struct (`SkippedUnknownRow`)
- Aligned Rust models with DB schema (nullable fields support)
- Improved consistency between provisioning, migrations, and runtime schema

---

## [0.9.7] - 2026-03-20

### 🐛 Fixed

- Fixed crash in `unknown list` when encountering records with NULL `fid`
- Updated `UnknownPlanet.fid` to `Option<i64>` to properly support nullable database values
- Fixed formatting issues when printing optional coordinates (`x`, `y`) in CLI output
- Fixed invalid formatting specifiers for optional values in `unknown list`

### 🔄 Improved

- Improved robustness of unknown planet handling with incomplete data (missing `fid`, `x`, `y`)
- Added validation for coordinates before executing proximity search (`unknown search`, `near`)
- Improved CLI error messages for missing coordinates

### 🧩 Internal

- Aligned Rust models with database schema for `planets_unknown`
- Ensured consistent handling of nullable fields across DB layer and CLI

---

## [0.9.6] - 2026-03-20

### Added

- aligned `planets_unknown` with the `planets` schema to support future staging/edit workflows
- added an internal unknown-record `id` primary key plus `reviewed`, `promoted`, and `notes` workflow fields
- added normalized `planet_norm` storage for unknown rows to prepare future matching and editing commands

### Changed

- `unknown list` now displays both internal unknown `ID` and source `FID`
- `unknown search <id> --near <parsecs>` now resolves nearby planets from the unknown table using the internal unknown
  record id
- database provisioning and incremental updates now populate the expanded `planets_unknown` staging schema
- Added pagination support to `unknown list` with `--page` and `--page-size`
- bumped workspace version to `0.9.6`

### Fixed

- preserved the robust squared-distance SQL strategy for nearby unknown-planet search while moving to the staged
  unknown-record model

---

## [0.9.5] - 2026-03-20

### Added

- added `unknown list` to display records from `planets_unknown`
- added `unknown search <fid> --near <parsecs>` to find known planets near an unknown planet (superseded in `0.9.6` by
  internal unknown record IDs)
- added core query helpers for listing unknown planets and resolving nearby known planets from unknown coordinates

### Changed

- nearby search for unknown planets uses the robust squared-distance SQL strategy based on `x`/`y` coordinates
- bumped workspace version to `0.9.5`

---

## [0.9.1] - 2026-03-19

### Fixed

- made the GUI more robust when the CLI sibling executable is not available
- restored the historical CLI binary name `sw_galaxy_map` to avoid breaking existing scripts
- vendored the GUI icon inside the `sw_galaxy_map_gui` crate so packaging and `cargo package` work correctly
- added a CLI library entry point for GUI help integration
- improved GUI command/help fallback behavior after the workspace split

### Changed

- refined post-`0.9.0` workspace split integration between `sw_galaxy_map_cli` and `sw_galaxy_map_gui`
- updated packaging layout to keep GUI assets self-contained inside the GUI crate

---

## [0.9.0] - Unreleased

### Changed

- Project reorganized as a Cargo workspace with three crates:
    - `sw_galaxy_map_core`
    - `sw_galaxy_map_cli`
    - `sw_galaxy_map_gui`
- `cargo run -p sw_galaxy_map_cli` now always starts only the CLI.
- `cargo run -p sw_galaxy_map_gui` now always starts only the GUI.
- Removed the legacy startup discriminant used to switch from CLI to GUI automatically.
- Updated workspace migration notes and README to document the explicit CLI/GUI entrypoints.

---

## v0.8.2 — 2026-02-06

### Added

- Optional **sublight travel time estimation** (km/s) alongside hyperspace ETA.
- `waypoint prune --include-linked` to remove orphan `computed` waypoints even when linked to planets (links are removed
  as part of prune).

### Changed

- **Startup behavior adjusted**:
    - Running `sw_galaxy_map` with **no arguments** now launches the **GUI**.
    - The **CLI must be explicitly requested** using the `--cli` flag.
- Refactored route polyline length computation into a shared helper to remove duplication.
- Improved `waypoint list` UX:
    - orphan marker (`*`) for linked waypoints not used by any route
    - conditional legend line printed only when needed
    - stable table layout with truncation and padding.

### Fixed

- Clippy/lint cleanups under `-D warnings` (precision constants, unused helpers/models).

---

## v0.8.1 — 2026-02-10

### Added

- `route compute` now accepts two or more planets to compute multi-leg trips in one command.

---

## v0.8.0 — 2026-01-28

### ⚠️ Breaking change

- **Default startup behavior changed**: running `sw_galaxy_map` with **no arguments** now enters **Interactive CLI**
  mode.  
  Use `--gui` to start the GUI.

### Added

- **Interactive CLI mode** when no args are provided.
- `--gui` flag to explicitly start the GUI.
- `route explain`: new options
    - `--class <f64>` to set Hyperdrive class for ETA computation
    - `--region-blend <avg|conservative|w>` to control region compression blending.
- **ETA summary** in `route show` (defaults: class `1.0`, blend `avg`).
- `waypoint links <id>` now also shows **Associated routes** for the waypoint.
- `waypoint prune` to remove orphan `computed` waypoints:
    - `--dry-run` preview mode
    - `--include-linked` to prune even if linked to planets (links are removed as part of prune).
- New utility module `src/utils/formatting.rs`:
    - `truncate_ellipsis()` for stable table rendering
    - `print_kv_block_colored_keys()` for aligned `key: value` blocks (multiline-safe).

### Changed

- Improved table rendering for `route list`:
    - consistent alignment
    - `FROM/TO` truncation to prevent table breaks
    - “found N routes …” summary with LIMIT awareness
    - compact English messaging.
- Improved `waypoint list`:
    - shows total count + paging behavior
    - shows `LINKS` count and an **orphan marker** for “linked but unused by any route”
    - optional legend line printed only when needed.

### Fixed

- Region parsing/selection issues in `route explain` ETA output (endpoint regions are now handled consistently).
- Various CLI plumbing issues introduced while restructuring clap + command dispatch (DB opening / command routing).

---

## [0.7.5] – 2026-01-26

### ✨ New features

- Added hyperspace **ETA estimation** for routes based on:
    - geometric route length (polyline, including detours)
    - galactic region compression factors
    - hyperdrive class scaling
- `route show` now displays a **synthetic ETA** using default parameters.
- `route explain` supports ETA customization via:
    - `--class <f64>` (hyperdrive class)
    - `--region-blend avg|conservative|<weight>`

### 🧭 ETA model

- Introduced a reusable ETA computation model with:
    - region-based hyperspace compression
    - weighted blending for multi-region routes
    - soft detour penalties (count + severity)
- Hyperdrive class semantics fixed:
    - lower class values are faster (e.g. class 0.5 is twice as fast as class 1.0)

### 🛠 Improvements

- Refactored galactic region extraction into a shared helper.
- Debug-only logging for raw region metadata (`debug_assertions` gated).
- Clear separation between routing cost and navigational ETA estimation.

### 🧪 Stability

- ETA computation is fail-soft: missing data does not break route display.
- No database schema changes.

---

## [0.7.2] - 2026-01-26

### ✨ New Features

#### GUI Console Mode

- Introduced a **console-style GUI** that accepts the same commands as the CLI
- A single `CMD` input box executes native CLI commands (e.g. `search`, `near`, `route`, `db`)
- Commands are executed by spawning the current executable, ensuring **full CLI feature parity**
- Standard output and error streams are captured and rendered in the GUI output panel
- JSON output is automatically detected and can be exported to file

#### Integrated Help System

- Added a dedicated **Help window** (F1 / Help button)
- Help content is generated by running real CLI commands:
    - `--help`
    - `route --help`
    - `search --help`, etc.
- Guarantees that GUI help is always **consistent with CLI documentation**

#### System Status & Feedback

- Persistent **status bar** for system and operation messages
- Diegetic **navicomputer bootstrap sequence** displayed at startup
- Database connection indicator:
    - Green dot = connected
    - Red dot = error
    - Tooltip shows detailed DB status
- Status messages support automatic TTL (time-to-live) for transient events

---

### 🛠 Improvements

#### Command Validation

- Unified validation logic across CLI and GUI for:
    - `search`
    - `near`
    - `route`
- Explicit support for negative coordinates using `--x=-190` / `--y=-190`
- Validation error messages are reused consistently in both CLI and GUI

#### Routing & Queries

- Extended `route list` with a new filter:
    - `--wp <count>` — filter routes by exact number of waypoints
- Improved tabular formatting and column alignment for:
    - `search`
    - `near`
    - `info`
- Output is now more readable and stable across terminals

#### Database Migrations

- Refactored database migrations into a **dynamic, incremental engine**
- Migrations are applied sequentially using a versioned step registry
- Added support for:
    - `db migrate --dry-run` (no changes applied)
- Automatic migrations are now **silent when no changes are required**
- Final summary reports how many migrations were applied

---

### 🧹 Fixes

- Fixed text selection issues in GUI text areas:
    - Right-click no longer clears the current selection
    - Drag-based selection works correctly
- Resolved build failures in `--release` mode:
    - Debug-only routing helpers are now properly gated
- Removed redundant schema status messages during normal command execution

---

### 🔒 Stability & Compatibility

- No regressions in CLI behavior
- GUI starts **only when the application is launched without arguments**
- Fully compatible with database schema version **v10**

---

## [0.7.1] – 2026-01-24

### Added

- Embedded application icons for Windows and Linux.
- GUI status bar improvements: automatic message reset to a ready state after a short timeout.
- Input validation in GUI: normalized FROM/TO, prevented FROM==TO and empty inputs.
- UI tooltips for version, database indicator, and main actions.

### Changed

- Minor UX polish: clearer status messaging and more consistent user feedback.

---

## [0.7.0] – 2026-01-22

### Added

- Introduced a graphical Navicomputer UI (GUI), launched automatically when no CLI arguments are provided.
- FROM / TO input fields with keyboard-friendly behavior.
- Compute and Clear actions.
- Scrollable, monospace output area for route results.
- JSON export via file dialog with standard filename format.
- Dedicated status bar with diegetic boot sequence and system messages.

### Changed

- Application startup logic now selects CLI or GUI mode depending on provided arguments.

### Notes

- CLI functionality remains fully backward-compatible and unchanged.

---

## [0.6.2] – 2026-01-20

### ✨ New Features

- Added `route list` command to list persisted routes
- Introduced advanced filtering options for `route list`:
    - `--status <status>`
    - `--from <planet_fid>`
    - `--to <planet_fid>`
- Added configurable sorting for route listing via `--sort`:
    - `updated` (default)
    - `id`
    - `length`
- Added JSON export support for route listing:
    - `route list --json`
    - `route list --json --file <path>`

### 🎨 UX Improvements

- Colorized `route list` output using the unified CLI color policy
- Clear visual distinction between:
    - successful routes
    - routes with detours
    - zero-count values (dimmed)
- Consistent behavior between stdout JSON export and file-based export

### 🛠 Internal / Refactoring

- Introduced `RouteListOptions` struct to group list parameters and satisfy Clippy constraints
- Refactored dynamic SQL generation for route listing with:
    - safe, optional `WHERE` clauses
    - whitelisted `ORDER BY` clauses
- Fixed SQL parameter binding mismatch in dynamically generated queries
- Improved robustness of file output handling (parent directory creation)

### 🐛 Bug Fixes

- Fixed incorrect SQL parameter count when using filtered `route list`
- Fixed path parent handling when exporting JSON to file

### ⚠️ Notes

- `route list` is backward-compatible with existing databases
- JSON export produces clean machine-readable output with no extra text on stdout

---

## [0.6.1] – 2026-01-20

### ✨ New Commands

- Added `route clear` command to delete all persisted routing data:
    - `routes`
    - `route_waypoints`
    - `route_detours`
- Added interactive confirmation for destructive operations, with optional `--yes` flag for non-interactive usage
- Added `route prune` command to remove orphan rows in routing tables
  (`route_waypoints` and `route_detours` not linked to any route)

### 🎨 UX Improvements

- Introduced colorized output for `route clear` and `route prune`, consistent with the unified CLI color policy:
    - destructive actions highlighted in red
    - successful operations in green
    - zero-effect operations dimmed
    - partial cleanups highlighted in yellow
- Improved user feedback for aborted destructive operations

### 🛠 Internal / Refactoring

- Reused centralized `confirm_destructive()` helper to avoid duplicated confirmation logic
- Ensured routing cleanup commands never affect galaxy/domain data
  (`waypoints`, `waypoint_planets`, `planets` remain untouched)
- Improved robustness of maintenance commands against disabled foreign key constraints

### ⚠️ Notes

- `route prune` is a safe housekeeping operation and does not require confirmation
- Both commands are backward-compatible and do not affect existing route computation logic

---

## [0.6.0] – 2026-01-20

### ✨ New Features

- Introduced **explainable routing** with the new `route explain <id>` command
- Added **detour telemetry** (`tries_used`, `tries_exhausted`) to make routing decisions provable
- Implemented **JSON export** for route explanations via `route explain --json`
- Added support for **file output** with `--file <path>` when exporting JSON
- Unified and centralized **CLI color policy** across `route show` and `route explain`
- Enhanced CLI output with **context-aware coloring** (start/end, obstacles, detours, scores)
- Added explanatory footer notes to clarify routing invariants and units

### 🧠 Improvements

- Refactored routing engine to propagate decision telemetry from runtime to persistence
- Improved detour scoring diagnostics with dominant penalty identification
- Hardened route explanation against legacy routes (partial telemetry)
- Improved robustness of DB loading for extended detour metadata

### 🛠 Internal / Refactoring

- Added shared `cli::color` helpers to avoid duplicated color logic
- Cleaned up complex CLI output code paths to avoid temporary borrow issues
- Improved route explanation structure for future machine consumption (JSON schema-ready)

### ⚠️ Backward Compatibility

- Existing routes computed before v0.6.0 remain readable and explainable
  (telemetry fields may be reported as `n/a`)

---

## [0.5.3] – 2026-01-19

### ✨ Routing & Persistence

- Stabilized routing engine (router_v1) with robust detour handling
- Full support for:
    - angular penalty (turn_weight)
    - backtracking penalty
    - proximity penalty to other planets
- Default `safety = 2.0 parsecs` validated on real routes

### 🧭 Computed Waypoints

- Persistence of detour waypoints as `computed`
- Deterministic fingerprint-based deduplication
- Automatic waypoint → obstacle planet linking with role `avoid`

### 🗺️ Persisted Routes

- Route upsert per (from → to) planet pair
- Existing routes are updated when routing parameters change
- Full persistence of:
    - route polyline (`route_waypoints`)
    - detailed detour data (`route_detours`)
    - score breakdown for each detour decision

### 🔍 Improved `route show`

- Display planet names (start / end) instead of raw FIDs
- Direct JOINs to retrieve:
    - obstacle planet names in detours
    - associated waypoint names
- Semantic labels:
    - `Start` / `End` instead of `wp_id=-`
- Cleaner output, ready for future visualizations

### 🧪 Tests

- Routing integration tests:
    - direct route
    - single obstacle
    - multiple obstacles
- Shared helper `assert_collision_free`

### 📚 Documentation

- Documented:
    - meaning and usage of the `--safety` parameter
    - detour waypoint selection algorithm
    - database design for routes and computed waypoints

### 🧹 Misc

- Clippy clean (warning-free)
- Cleanup of legacy and unused APIs

---

## [0.5.2] – 2026-01-19

### ✨ Routing & Persistence

- Introduced full persistence for computed routes (schema v7–v8)
- Routes are now **unique per (from, to)** pair and automatically **upserted**
- Re-running a route with different parameters updates the existing record instead of creating duplicates
- Added support for storing:
    - route metadata (options, length, iterations, status)
    - route polyline (ordered waypoints)
    - detailed detour decisions with scoring breakdown
- Computed detour waypoints are deduplicated via fingerprint and stored in the global waypoint catalog

### 🧭 CLI Enhancements

- Refactored `route` command into subcommands:
    - `route compute <from> <to>`
    - `route last <from> <to>`
    - `route show <route_id>`
- Added inspection commands for persisted routes
- Improved routing debug output (detours, scoring, geometry)

### 🧪 Tests

- Added integration tests for routing:
    - direct route without obstacles
    - single obstacle detour
    - multiple obstacles
- Introduced shared collision-free assertion helper for routes

### 🗄️ Database

- Schema upgrades up to **v8**
- Added `routes`, `route_waypoints`, `route_detours`
- Enforced uniqueness on `(from_planet_fid, to_planet_fid)`
- Added `updated_at` to routes for proper cache semantics

### 🧹 Internal

- Routing code refactored and modularized
- Clippy clean (`-D warnings`)
- Improved separation between routing logic, persistence, and CLI

---

## [0.5.1] – 2026-01-19

### 🚀 Routing Engine (in-memory, v1)

- Implemented first working routing engine between two planets using Euclidean geometry.
- Added incremental detour-based routing with obstacle avoidance.
- Introduced waypoint-based route construction with iterative refinement.
- Added scoring system for detour candidates, including:
    - path length
    - angular penalty
    - backward movement penalty
    - proximity penalty to other planets

### 🧠 Collision Handling

- Distinguished between:
    - **hard collisions** (segment intersects obstacle interior)
    - **endpoint collisions** (allowed for start/end planets).
- Fixed false-positive collisions when arriving at destination planet.
- Unified collision logic across detection and validation (`first_collision_on_segment` and `is_segment_safe`).

### 🔍 Debug & Observability

- Added detailed debug output for failed detour resolution, including:
    - obstacle data
    - closest-point metrics
    - candidate waypoint diagnostics.
- Improved internal diagnostics to support future visualization and persistence.

### 🗺️ CLI

- Added `route <FROM> <TO>` command (planet name or alias).
- Integrated routing engine with database-backed planet resolution.
- Added tunable routing parameters via CLI flags (safety, clearance, scoring weights).

### 🧱 Internal Refactor

- Separated routing logic into dedicated `routing` module.
- Improved internal consistency and robustness of geometric primitives.
- Prepared groundwork for waypoint persistence and multi-hop routing.

---

## [0.5.0] – 2026-01-16

### Added

- Introduced the Waypoints catalog (schema v5) to store reusable navigation waypoints.
- Added waypoint CRUD queries (insert/find/list/delete) using column-name based mapping.
- Added CLI `waypoint` command group:
    - `waypoint add`
    - `waypoint list`
    - `waypoint show`
    - `waypoint delete`

### Changed

- Moved DB-related sources under `src/db/` for clearer separation of concerns (provisioning vs database layer).
- Improved schema migration workflow with incremental migration steps and user-facing progress messages.

### Fixed

- Fixed schema migration correctness issues (ensuring migrations are applied before updating `meta.schema_version`).
- Fixed CLI parsing for negative coordinate values in waypoint commands.
- Resolved Clippy warnings to keep `cargo clippy -- -D warnings` clean.

---

## [0.4.1] – 2026-01-16

### Changed

- Refactored the database layer to consolidate all planet-related queries into `db/queries.rs`.
- Removed duplicated and legacy planet lookup logic from the former `db.rs` (now `db/core.rs`).
- Switched `Planet` row mapping to column-name based access (`row.get("...")`) for improved robustness.
- Standardized SQL queries using canonical column aliases shared across direct and alias-based lookups.
- Reorganized the `db` module structure (`db/mod.rs`, `db/core.rs`, `db/queries.rs`) for better maintainability.

### Added

- Added a derived Star Wars Fandom information URL for planets, exposed via `Planet::info_planet_url()`.
- Included the Fandom URL in the output of the `info` command.

### Fixed

- Fixed invalid column name errors caused by inconsistent SQL aliases (`ref` vs `reference`).
- Resolved compilation issues related to mixed `anyhow::Result` / `rusqlite::Result` usage in query closures.
- Fixed all Clippy warnings, ensuring a clean `cargo clippy -- -D warnings` run.

### Internal

- Improved separation of concerns between DB connection/setup logic and query logic.
- Reduced the risk of future SQL drift by enforcing a single source of truth for planet queries.

---

## v0.4.0 – Incremental database updates

### Added

- New `db update` command with fully incremental update logic.
- Hash-based comparison (`arcgis_hash`) to detect changed planets.
- Soft-delete support via `deleted` column on `planets`.
- Optional `--prune` flag to permanently remove deleted planets.
- `--dry-run` mode to preview changes without writing to disk.
- `--stats` flag to display update statistics and top changed FIDs.
- Extended `db status` with ArcGIS layer metadata and edit timestamps.
- Automatic FTS5 rebuild after updates (when enabled).

### Improved

- More robust handling of invalid ArcGIS rows.
- Clearer CLI output with colored messages and counters.
- Consistent metadata tracking for update mode and pruning.

### Internal

- Schema version bump.
- Incremental update logic isolated and transaction-safe.
- Shared normalization and hash computation reused across init/update.

---

## [0.3.0] – 2026-01-15

### Added

- Unified colored console messaging system with semantic levels:
    - info (cyan), success (green), warning (yellow), error (red).
- Emoji support for user-facing messages (ℹ️ ✅ ⚠️ ❌).
- Automatic color disabling when stdout is not a TTY.
- Centralized UI output handling, decoupled from domain logic.

### Changed

- Refactored CLI output to use the new UI messaging layer.
- Simplified console output by removing explicit log tags (e.g. [INFO], [ERROR]).
- Improved consistency of user-facing messages across all commands.

### Fixed

- Resolved type mismatch issues related to colored output handling.
- Ensured proper error propagation with `anyhow` while keeping UI concerns isolated.

### Notes

- Error handling continues to rely on `anyhow::Result` and `context()` internally.
- Colored output is applied only at the CLI entrypoint level.

---

## [0.2.0] – 2026-01-15

### Added

- Automatic local database initialization on first use (`search`, `info`, `near`) if the database is missing.
- `db status` command to inspect local database path, metadata, counts, schema and FTS status.
- Full-Text Search (FTS5) support with automatic detection and fallback to indexed LIKE search.
- Normalized search table (`planet_search`) and FTS-backed search (`planets_fts`) when available.
- Alias-based planet lookup (name0/name1/name2).

### Improved

- Database provisioning moved to OS local application data directory.
- Search relevance and performance improved via FTS5 (`bm25`) when supported.
- CLI UX improvements for `db init`:
    - interactive overwrite confirmation when the database already exists
    - `--force` to bypass confirmation.
- Robust handling of invalid source records (missing Planet or X/Y).

### Fixed

- Dependency compatibility with `reqwest 0.13` using `rustls-tls-webpki-roots`.
- Clippy warnings and type-complexity issues resolved.

### Notes

- Some source records are intentionally skipped during import if required fields are missing.
- FTS5 availability depends on the SQLite build; fallback search is always available.

---

## [0.1.0] - 2026-01-15

### Added

- Initial release of the **sw_galaxy_map** CLI application.
- SQLite-backed local database for offline planet queries.
- Text-based planet search using normalized names and aliases.
- Planet detail command displaying all available information.
- Nearby planet search within a given radius using Euclidean distance
  on X/Y coordinates expressed in parsecs.
- Support for alias-based lookup derived from multiple known planet names.
- Clear attribution and acknowledgements for the original data source
  (Star Wars Galaxy Map by Henry Bernberg: read the [README](README.md) for further information).

### Notes

- This is the first public version of the project and should be considered
  an initial, evolving release.
- The database is intended for local, educational, and non-commercial use.
