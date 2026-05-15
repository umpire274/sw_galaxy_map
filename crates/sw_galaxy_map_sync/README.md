# sw_galaxy_map_sync

ArcGIS-first synchronization, ingestion and normalization engine for the `sw_galaxy_map` workspace and the future
Astronavis™ ecosystem.

`sw_galaxy_map_sync` is responsible for:

* importing canonical ArcGIS galaxy-map data,
* preserving unknown and unnamed source records,
* applying curated official CSV overlays,
* synchronizing SQLite and PostgreSQL databases,
* maintaining normalized multi-backend datasets,
* preparing future API and analytics pipelines.

---

# Highlights

Version `0.4.0` introduces:

* standalone schema provisioning,
* PostgreSQL backend support,
* async synchronization pipelines,
* backend-aware CLI architecture,
* structured progress bars with ETA reporting,
* metadata persistence for sync operations,
* curated overlay reconciliation,
* synthetic negative FID support.

---

# Architecture

```text
ArcGIS FeatureServer
        │
        ▼
sw_galaxy_map_core::provision::arcgis
        │
        ▼
sw_galaxy_map_sync
        │
        ├── Schema bootstrap
        │       ├── SQLite v13
        │       └── PostgreSQL v13
        │
        ├── ArcGIS baseline import
        │       ├── planets
        │       └── planets_unknown
        │
        ├── Curated CSV overlay
        │       └── planets.status
        │
        ├── Metadata persistence
        │       └── meta
        │
        └── Future normalization pipelines
                └── pc → ly conversion
```

The crate intentionally reuses the ArcGIS provider already implemented in:

```rust
sw_galaxy_map_core::provision::arcgis
```

However, starting from `v0.4.0`, schema provisioning is fully standalone inside:

```text
sw_galaxy_map_sync::db::schema
```

This avoids runtime coupling with the core database provisioning layer.

---

# Backend Support

## SQLite

SQLite is the default backend.

Recommended for:

* local development,
* CLI usage,
* GUI usage,
* portable datasets,
* offline workflows.

---

## PostgreSQL

PostgreSQL support was introduced in `v0.4.0`.

Recommended for:

* centralized synchronization,
* remote servers,
* future Astronavis™ APIs,
* multi-user environments,
* analytics and reporting.

The PostgreSQL backend supports:

* schema bootstrap,
* ArcGIS import,
* CSV overlay synchronization,
* dry-run validation,
* metadata persistence,
* progress reporting.

---

# Database Philosophy

The synchronization pipeline is intentionally split into multiple logical layers.

## 1. ArcGIS Baseline

ArcGIS is treated as the canonical coordinate and source-data baseline.

This layer provides:

* authoritative ArcGIS FIDs,
* X/Y coordinates,
* canonical source metadata,
* unknown/unnamed records,
* stable synchronization hashes.

---

## 2. Curated Official CSV Overlay

The curated CSV overlay is treated as the canonical naming and metadata validation layer.

This layer provides:

* official naming validation,
* curated Region/Sector/Grid corrections,
* curated-only systems,
* logical deletion handling,
* overlay status tracking.

---

## 3. Metadata Tracking

Synchronization operations persist metadata into:

```text
meta
```

Examples:

* schema bootstrap information,
* ArcGIS import summaries,
* CSV overlay statistics,
* synchronization timestamps,
* backend information.

---

# FID Convention

`sw_galaxy_map_sync` intentionally separates authoritative and synthetic records.

## Positive FIDs

```text
FID > 0
```

Positive identifiers represent authoritative ArcGIS records.

These values originate from the ArcGIS source dataset.

---

## Negative FIDs

```text
FID < 0
```

Negative identifiers represent synthetic/local overlay records.

These records are generated when:

* a curated CSV row does not exist in ArcGIS,
* a local-only system must be preserved,
* a future enrichment pipeline creates synthetic entities.

This convention prevents collisions with future ArcGIS imports.

---

# Coordinate Policy

Coordinates are currently stored internally as:

```text
parsecs (pc)
```

The synchronization pipeline stores:

```text
grid_unit = 'pc'
```

for all imported ArcGIS records.

Current policy:

```text
DB canonical coordinates : parsecs
UI/export coordinates    : light years
conversion               : ly = pc × 3.26156
```

Future releases will introduce:

```text
pc → ly normalization pipelines
```

inside the sync crate itself.

---

# Unknown ArcGIS Records

ArcGIS occasionally exposes records without a valid or usable planet name.

These records are intentionally preserved instead of discarded.

They are stored inside:

```text
planets_unknown
```

with:

* original ArcGIS FID,
* source metadata,
* coordinates,
* synchronization hashes,
* canonical/legends flags.

This enables:

* future inspection,
* promotion,
* manual correction,
* coordinate reconciliation,
* historical preservation.

---

# Progress Bars

Long-running synchronization operations use structured progress bars.

Features include:

* percentage reporting,
* ETA estimation,
* elapsed-time rendering,
* multi-phase synchronization reporting.

Example:

```text
[00:12] [##############--------------] 2450/4654 ( 52%) ETA:00:11 Applying CSV overlay...
```

Progress bars are supported for:

* ArcGIS import,
* CSV overlay,
* delete reconciliation,
* dry-run validation.

---

# Supported Backends

## SQLite Commands

SQLite uses:

```text
--driver sqlite
--db <database.sqlite>
```

---

## PostgreSQL Commands

PostgreSQL uses:

```text
--driver postgres
--db-config <config.json>
```

Example configuration:

```json
{
  "host": "127.0.0.1",
  "port": 5432,
  "database": "astronavis",
  "username": "astronavis_app",
  "password": "strong_password"
}
```

---

# Quick Start

## SQLite Bootstrap

```bash
cargo run -p sw_galaxy_map_sync -- db bootstrap \
  --driver sqlite \
  --db res/sw_planets.sqlite
```

---

## PostgreSQL Bootstrap

```bash
cargo run -p sw_galaxy_map_sync -- db bootstrap \
  --driver postgres \
  --db-config res/config.postgres.json
```

---

# ArcGIS Pipeline

## SQLite Import

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import \
  --driver sqlite \
  --db res/sw_planets.sqlite
```

---

## PostgreSQL Import

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import \
  --driver postgres \
  --db-config res/config.postgres.json
```

---

## Dry Run

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import \
  --driver postgres \
  --db-config res/config.postgres.json \
  --dry-run
```

---

# CSV Overlay Pipeline

The synchronization workflow is intentionally layered:

```text
1. ArcGIS import
   ArcGIS → planets / planets_unknown

2. CSV overlay
   curated CSV → metadata/status reconciliation
```

---

# Supported CSV Formats

## Official Curated CSV

```csv
system;sector;region;grid
Adelphi;Kibilini;Outer Rim Territories;P-17
23 Mere;;Colonies;M-13
```

Default delimiter:

```text
;
```

---

## Full Export CSV

```csv
FID,Planet,planet_norm,Region,Sector,System,Grid,X,Y,...
```

Default delimiter:

```text
,
```

---

# SQLite CSV Overlay

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --driver sqlite \
  --db res/sw_planets.sqlite \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv \
  --format official \
  --delimiter ";"
```

---

# PostgreSQL CSV Overlay

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --driver postgres \
  --db-config res/config.postgres.json \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv \
  --format official \
  --delimiter ";"
```

---

# CSV Overlay Options

## Dry Run

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --driver postgres \
  --db-config res/config.postgres.json \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv \
  --format official \
  --delimiter ";" \
  --dry-run
```

---

## Mark Deleted

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --driver postgres \
  --db-config res/config.postgres.json \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv \
  --format official \
  --delimiter ";" \
  --mark-deleted
```

---

# Overlay Matching Logic

For every CSV row, the overlay pipeline attempts:

1. exact planet-name match,
2. Roman suffix/base-name fallback match,
3. curated-only insertion when no match exists.

Examples:

```text
Yavin      ↔ Yavin IV
Yavin IV   ↔ Yavin
```

---

# Overlay Status Values

| Status     | Meaning                                                         |
|------------|-----------------------------------------------------------------|
| `active`   | CSV row matches the existing DB metadata                        |
| `modified` | CSV row updated metadata for an existing DB row                 |
| `inserted` | CSV row inserted as a curated-only synthetic row                |
| `deleted`  | DB row is not represented in the CSV overlay                    |
| `skipped`  | Row was represented logically but required no status transition |

---

# Schema Provisioning

Starting from `v0.4.0`, schema provisioning is fully standalone.

Supported schema features include:

* SQLite v13 schema provisioning,
* PostgreSQL v13-aligned provisioning,
* automatic index creation,
* view provisioning,
* metadata initialization,
* future normalization support.

The sync crate no longer depends on:

```text
sw_galaxy_map_core::db::provision
```

for schema creation.

---

# Suggested Tests

## Build

```bash
cargo build -p sw_galaxy_map_sync
```

---

## Format

```bash
cargo fmt --all -- --check
```

---

## Clippy

```bash
cargo clippy -p sw_galaxy_map_sync --all-targets --all-features -- -D warnings
```

---

## Tests

```bash
cargo test -p sw_galaxy_map_sync
```

---

# Recommended End-to-End SQLite Test

```powershell
Remove-Item .\res\sw_planets_overlay_test.sqlite -ErrorAction SilentlyContinue

cargo run -p sw_galaxy_map_sync -- db bootstrap `
  --driver sqlite `
  --db res/sw_planets_overlay_test.sqlite

cargo run -p sw_galaxy_map_sync -- arcgis import `
  --driver sqlite `
  --db res/sw_planets_overlay_test.sqlite

cargo run -p sw_galaxy_map_sync -- csv overlay `
  --driver sqlite `
  --db res/sw_planets_overlay_test.sqlite `
  --csv res/star_wars_galaxy_systems_official_semicolon.csv `
  --format official `
  --delimiter ";" `
  --mark-deleted
```

---

# Recommended End-to-End PostgreSQL Test

```powershell
cargo run -p sw_galaxy_map_sync -- db bootstrap `
  --driver postgres `
  --db-config res/config.postgres.json

cargo run -p sw_galaxy_map_sync -- arcgis import `
  --driver postgres `
  --db-config res/config.postgres.json

cargo run -p sw_galaxy_map_sync -- csv overlay `
  --driver postgres `
  --db-config res/config.postgres.json `
  --csv res/star_wars_galaxy_systems_official_semicolon.csv `
  --format official `
  --delimiter ";" `
  --mark-deleted
```

---

# Future Roadmap

Planned future milestones:

```text
v0.5.x  Coordinate normalization pipelines
v0.6.x  Astronavis API integration
v0.7.x  Bulk synchronization optimization
v0.8.x  Advanced analytics and reconciliation
```

Potential future features:

* pc → ly normalization,
* batch PostgreSQL synchronization,
* COPY-based bulk import,
* MySQL backend,
* route analytics,
* synchronization diff reports.

---

# License

Licensed under either:

* MIT License
* Apache License 2.0

at your option.

---

# Branding Notice

Astronavis™ branding assets, logos, icons, wallpapers and visual identity materials are not covered by the open-source
licenses and remain proprietary.
