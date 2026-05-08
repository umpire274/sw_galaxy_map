# sw_galaxy_map_sync

ArcGIS-first synchronization and ingestion tool for the `sw_galaxy_map` workspace and the future Astronavis™ data
pipeline.

`sw_galaxy_map_sync` is responsible for:

* importing and normalizing ArcGIS data,
* preserving unnamed ArcGIS records,
* synchronizing curated official CSV overlays,
* maintaining the SQLite baseline used by the CLI, GUI and future API layers.

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
        ├── ArcGIS baseline import
        │       ├── planets
        │       └── planets_unknown
        │
        └── Curated CSV overlay
                └── planets.status
```

The crate intentionally reuses the ArcGIS provider already implemented in:

```rust
sw_galaxy_map_core::provision::arcgis
```

This avoids duplicated source logic and keeps ArcGIS handling centralized inside the core crate.

---

# Data Philosophy

The synchronization pipeline is intentionally split into two layers.

## 1. ArcGIS Baseline

ArcGIS is treated as the canonical coordinate and source-data baseline.

This layer provides:

* stable FID identifiers,
* X/Y coordinates,
* raw source metadata,
* unknown or unnamed records,
* normalized ArcGIS ingestion.

## 2. Curated Official CSV Overlay

The curated CSV overlay is treated as the canonical naming and metadata validation layer.

This layer provides:

* official naming validation,
* curated Region/Sector/Grid corrections,
* status tracking,
* logical deletion handling,
* curated-only systems missing from ArcGIS.

This design allows Astronavis™ to preserve:

* stable coordinates,
* curated naming,
* historical ArcGIS data,
* unknown records,
* future manual corrections.

---

# Current Features

## ArcGIS Pipeline

* Download planet records from ArcGIS.
* Normalize ArcGIS attributes into stable internal models.
* Export normalized ArcGIS data to JSON.
* Split ArcGIS records into:

    * known planets,
    * unnamed/unknown records.
* Import/upsert known records into `planets`.
* Import/upsert unnamed records into `planets_unknown`.
* Auto-create `planets_unknown` when missing.
* Preserve unknown ArcGIS source records.
* Stable FID + hash upsert logic.
* `--dry-run` support.

## CSV Overlay Pipeline

* Import curated Disney/official CSV overlays.
* Support semicolon-separated official CSV files.
* Support full planets/export CSV files.
* Exact planet-name matching.
* Roman suffix/base-name fallback matching.
* Curated-only row insertion.
* Overlay status tracking.
* Logical deletion support.
* Dry-run overlay validation.

---

# Coordinate Policy

ArcGIS X/Y values are currently treated as the canonical stored coordinates.

Coordinates are currently stored as parsecs.

Values are rounded to 3 decimal places before insertion/update.

Recommended long-term policy:

```text
DB canonical coordinates: parsec
UI/export coordinates   : light years
conversion              : ly = pc * 3.26156
```

---

# Unknown ArcGIS Records

ArcGIS sometimes exposes records without a usable planet name.

These records are not discarded.

Instead, they are preserved in:

```text
planets_unknown
```

Records are stored with:

* original ArcGIS `FID`,
* original source metadata,
* available coordinates,
* ArcGIS source hash,
* canonical/legends flags.

This allows later:

* inspection,
* promotion,
* manual correction,
* future coordinate reconciliation.

---

# Quick Start

## 1. Import ArcGIS baseline

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import \
  --db res/sw_planets.sqlite
```

## 2. Overlay official CSV

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --db res/sw_planets.sqlite \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv
```

---

# Commands

## Show help

```bash
cargo run -p sw_galaxy_map_sync -- --help
```

## ArcGIS commands

```bash
cargo run -p sw_galaxy_map_sync -- arcgis --help
```

## CSV overlay commands

```bash
cargo run -p sw_galaxy_map_sync -- csv --help
```

---

# ArcGIS Commands

## Fetch ArcGIS JSON

Download ArcGIS data and export normalized records as JSON.

```bash
cargo run -p sw_galaxy_map_sync -- arcgis fetch \
  --out res/arcgis_planets.json
```

Optional page size:

```bash
cargo run -p sw_galaxy_map_sync -- arcgis fetch \
  --page-size 2000 \
  --out res/arcgis_planets.json
```

Generated JSON format:

```json
{
  "known": [
    {
      "fid": 2160,
      "planet": "Exegol",
      "planet_norm": "exegol",
      "region": "Unknown Regions",
      "sector": "...",
      "system": "...",
      "grid": "G-7",
      "x": 0.0,
      "y": 0.0,
      "arcgis_hash": "..."
    }
  ],
  "unknown": [
    {
      "fid": 372,
      "planet": "",
      "planet_norm": "",
      "region": "...",
      "sector": "...",
      "system": "...",
      "grid": "...",
      "x": 0.0,
      "y": 0.0,
      "arcgis_hash": "..."
    }
  ]
}
```

---

## Import ArcGIS into SQLite

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import \
  --db res/sw_planets.sqlite
```

Custom tables:

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import \
  --db res/sw_planets.sqlite \
  --table planets \
  --unknown-table planets_unknown
```

Dry run:

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import \
  --db res/sw_planets.sqlite \
  --dry-run
```

---

# CSV Overlay Pipeline

Version `0.3.1` introduces curated CSV overlays on top of the ArcGIS baseline.

Workflow:

```text
1. ArcGIS import
   ArcGIS → planets / planets_unknown

2. CSV overlay
   official CSV → planets status/update layer
```

---

# Supported CSV Formats

## Official semicolon CSV

Curated Disney/current official list format:

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

## Full planets/export CSV

Full table/export-style format:

```csv
FID,Planet,planet_norm,Region,Sector,System,Grid,X,Y,arcgis_hash,deleted,Canon,Legends,...
```

Default delimiter:

```text
,
```

---

# CSV Overlay Commands

## Dry run

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --db res/sw_planets_overlay_test.sqlite \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv \
  --format official \
  --delimiter ";" \
  --dry-run
```

## Real overlay

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --db res/sw_planets_overlay_test.sqlite \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv \
  --format official \
  --delimiter ";"
```

## Mark deleted

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --db res/sw_planets_overlay_test.sqlite \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv \
  --format official \
  --delimiter ";" \
  --mark-deleted
```

## Auto-detection mode

```bash
cargo run -p sw_galaxy_map_sync -- csv overlay \
  --db res/sw_planets_overlay_test.sqlite \
  --csv res/star_wars_galaxy_systems_official_semicolon.csv
```

---

# Overlay Matching Logic

For every CSV row, the overlay pipeline attempts:

1. exact planet match,
2. Roman suffix/base-name fallback match,
3. curated-only insertion when no match exists.

Examples:

```text
Yavin      ↔ Yavin IV
Yavin IV   ↔ Yavin
```

---

# Overlay Status Values

The overlay updates the `status` column using:

| Status     | Meaning                                                         |
|------------|-----------------------------------------------------------------|
| `active`   | CSV row matches the existing DB metadata                        |
| `modified` | CSV row updated metadata for an existing DB row                 |
| `inserted` | CSV row was inserted as a curated-only row                      |
| `deleted`  | DB row is not represented in the CSV overlay                    |
| `skipped`  | Row was represented logically but required no status transition |

---

# SQLite Expectations

The pipeline expects a `planets`-like table containing at least:

```sql
FID
INTEGER,
Planet TEXT,
planet_norm TEXT,
Region TEXT,
Sector TEXT,
System TEXT,
Grid TEXT,
X REAL,
Y REAL,
arcgis_hash TEXT,
deleted INTEGER,
Canon INTEGER,
Legends INTEGER,
status TEXT
```

The upsert logic is based on:

```text
FID + arcgis_hash
```

The implementation intentionally avoids `ON CONFLICT` to keep compatibility with:

```text
planets_unknown
```

without requiring PK constraints.

---

# Why sqlx/MySQL is not included yet

MySQL is part of the long-term Astronavis™ roadmap, but `v0.3.x` intentionally avoids adding `sqlx`.

Reasons:

* the workspace already standardizes on `rusqlite`,
* `sqlx/sqlite` can introduce `libsqlite3-sys` conflicts,
* the synchronization model must stabilize first.

Planned roadmap:

```text
v0.3.0  ArcGIS-first ingestion
v0.3.1  Curated CSV overlay pipeline
v0.4.0  Repository abstraction
v0.5.0  MySQL backend
v0.6.0  Astronavis API integration
```

---

# Suggested Tests

## Build

```bash
cargo build -p sw_galaxy_map_sync
```

## Format

```bash
cargo fmt --all -- --check
```

## Clippy

```bash
cargo clippy -p sw_galaxy_map_sync --all-targets --all-features -- -D warnings
```

## Unit tests

```bash
cargo test -p sw_galaxy_map_sync
```

---

# Recommended End-to-End Test

```powershell
Remove-Item .\res\sw_planets_overlay_test.sqlite -ErrorAction SilentlyContinue

cargo run -p sw_galaxy_map_sync -- arcgis import `
  --db res/sw_planets_overlay_test.sqlite

cargo run -p sw_galaxy_map_sync -- csv overlay `
  --db res/sw_planets_overlay_test.sqlite `
  --csv res/star_wars_galaxy_systems_official_semicolon.csv `
  --format official `
  --delimiter ";" `
  --dry-run

cargo run -p sw_galaxy_map_sync -- csv overlay `
  --db res/sw_planets_overlay_test.sqlite `
  --csv res/star_wars_galaxy_systems_official_semicolon.csv `
  --format official `
  --delimiter ";"

cargo run -p sw_galaxy_map_sync -- csv overlay `
  --db res/sw_planets_overlay_test.sqlite `
  --csv res/star_wars_galaxy_systems_official_semicolon.csv `
  --format official `
  --delimiter ";" `
  --mark-deleted
```

Expected overlay summary:

```text
Rows read
Inserted
Active
Modified
Deleted
Skipped
Dry run
```

---

# License

Licensed under either:

* MIT License
* Apache License 2.0

at your option.

---

# Branding Notice

Astronavis™ branding assets, logos, icons, wallpapers and visual identity materials are not covered by the open-source
licenses and remain pro
