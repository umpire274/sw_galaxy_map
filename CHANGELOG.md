# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
