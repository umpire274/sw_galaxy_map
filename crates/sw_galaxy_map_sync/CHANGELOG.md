# Changelog

All notable changes to `sw_galaxy_map_sync` will be documented in this file.

The format is based on Keep a Changelog
and this project follows Semantic Versioning.

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