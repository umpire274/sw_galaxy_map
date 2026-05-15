# Changelog

All notable changes to `sw_galaxy_map_sync` will be documented in this file.

The format is based on Keep a Changelog
and this project follows Semantic Versioning.

## [0.4.0] - 2026-05-13

### Added

- add PostgreSQL backend support
- add async PostgreSQL synchronization pipeline
- add backend-aware CLI architecture
- add `--driver` support across sync commands
- add PostgreSQL ArcGIS import pipeline
- add PostgreSQL CSV overlay pipeline
- add standalone SQLite schema provisioning
- add standalone PostgreSQL schema provisioning
- add v13-aligned schema support
- add PostgreSQL index and view provisioning
- add structured progress bar infrastructure
- add progress percentage reporting
- add ETA estimation support
- add elapsed-time progress reporting
- add unified progress rendering across all sync pipelines
- add metadata persistence for:
    - schema bootstrap
    - ArcGIS import
    - CSV overlay
- add synthetic negative FID generation for curated-only rows
- add standalone schema management independent from core
- add PostgreSQL dry-run support
- add PostgreSQL delete reconciliation support

### Changed

- redesign sync crate as a multi-backend synchronization engine
- decouple schema provisioning from `sw_galaxy_map_core`
- standardize backend selection through:
    - `--driver`
    - `--db`
    - `--db-config`
- align SQLite and PostgreSQL provisioning logic
- standardize progress reporting across all pipelines
- persist canonical coordinate unit as `grid_unit = 'pc'`

### Fixed

- fix PostgreSQL upsert constraint handling
- fix tokio runtime shutdown panic during SQLite execution
- fix overlay reconciliation inconsistencies
- fix duplicate schema provisioning logic
- fix CSV overlay progress reporting
- fix dry-run reconciliation behavior

---

## [0.3.2] - 2026-04-01

### Added

- add structured progress bar infrastructure using `indicatif`
- add synchronization progress reporting
- add deleted-row reconciliation progress reporting
- add shared progress bar helper utilities
- add elapsed-time rendering for long-running sync operations

### Changed

- improve synchronization pipeline UX
- standardize progress rendering across import operations
- prepare sync architecture for future multi-backend support

### Fixed

- improve long-running synchronization visibility
- improve feedback during delete reconciliation operations

---

## [0.3.1] - 2026-05-09

### Added

- add curated CSV overlay pipeline
- support official semicolon-separated Disney CSV
- support full planets/export CSV format
- add exact planet match logic
- add Roman suffix/base-name fallback matching
- add curated-only planet insertion
- add overlay status tracking:
    - active
    - modified
    - inserted
    - deleted
    - skipped
- add `--mark-deleted`
- add `--dry-run`
- add auto-detection for CSV format
- add CSV overlay summary reporting

### Changed

- redesign sync workflow around:
    - ArcGIS baseline
    - CSV overlay pipeline
- reuse SQLite repository layer for overlay operations
- improve normalized comparison helpers
- improve CLI sync integration

### Fixed

- fix NOT NULL insertion failures for curated-only rows
- fix overlay rerun idempotency
- fix suffix matching edge cases

---

## [0.3.0] - 2026-05-08

### Added

- redesign crate as ArcGIS-first ingestion pipeline
- reuse ArcGIS provider from `sw_galaxy_map_core`
- add normalized ArcGIS ingestion
- add JSON export support
- split records into:
    - planets
    - planets_unknown
- preserve unnamed ArcGIS records
- auto-create missing `planets_unknown`
- add stable FID + hash upsert logic
- add safe schema bootstrap handling
- add dry-run support

### Changed

- remove legacy CSV-first architecture
- remove experimental sqlx/MySQL integration
- standardize on rusqlite

### Fixed

- avoid destructive schema rebuilds
- fix ArcGIS unnamed record handling
- fix workspace sqlite dependency conflicts