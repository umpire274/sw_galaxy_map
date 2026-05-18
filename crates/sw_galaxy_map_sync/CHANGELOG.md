# Changelog

All notable changes to `sw_galaxy_map_sync` will be documented in this file.

The format is based on Keep a Changelog
and this project follows Semantic Versioning.

## [0.4.3] - 2026-05-18

### Added

- add coordinate normalization pipeline
- add `convert coordinates` command
- add pc ↔ ly coordinate conversion support
- add transactional coordinate backup infrastructure
- add `planets_coordinates_backup` snapshot table
- add interactive coordinate rollback workflow
- add backup selection interface for rollback operations
- add SQLite coordinate conversion support
- add PostgreSQL coordinate conversion support
- add SQLite coordinate rollback support
- add PostgreSQL coordinate rollback support
- add dry-run support for coordinate workflows
- add spinner-based progress feedback for bulk coordinate operations

### Changed

- standardize coordinate workflows across SQLite and PostgreSQL backends
- round normalized coordinates to two decimal places
- persist coordinate unit state through `grid_unit`

### Fixed

- prevent redundant coordinate conversions when target unit already matches database state
- improve rollback safety through transactional restore execution

---

## [0.4.2] - 2026-05-16

### Added

- add standalone ArcGIS FeatureServer access layer
- add standalone ArcGIS layer metadata fetching
- add standalone paginated ArcGIS feature retrieval
- add standalone ArcGIS error handling
- add internal ArcGIS paging infrastructure inside sync crate

### Changed

- move ArcGIS provider logic from `sw_galaxy_map_core` into `sw_galaxy_map_sync`
- remove dependency on `sw_galaxy_map_core::provision::arcgis`
- continue decoupling sync crate from core provisioning infrastructure
- keep ArcGIS normalization and known/unknown classification inside sync crate
- standardize standalone synchronization architecture

### Fixed

- clamp ArcGIS page size to layer `maxRecordCount`
- prevent truncated imports when oversized ArcGIS page sizes are requested
- preserve correct ArcGIS pagination behavior across multi-page imports

---

## [0.4.1] - 2026-05-15

### Fixed

- wrap PostgreSQL CSV overlay operations in a single transaction
- prevent partially applied overlay states on synchronization failures
- align PostgreSQL overlay atomicity with SQLite backend behavior
- include delete reconciliation and metadata persistence inside transactional execution
- replace raw PostgreSQL DSN string interpolation with `PgConnectOptions`
- correctly support PostgreSQL credentials containing reserved URL characters
- improve robustness of PostgreSQL backend configuration handling

---

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