# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
