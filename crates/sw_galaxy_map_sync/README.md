# sw_galaxy_map_sync

ArcGIS-first synchronization and ingestion tool for the `sw_galaxy_map` workspace and the future Astronavis data pipeline.

Version `0.3.0` resets the crate around a cleaner architecture:

```text
sw_galaxy_map_sync = orchestration CLI
sw_galaxy_map_core = ArcGIS source/provider logic
```

The crate does not duplicate ArcGIS logic. It reuses the already working provider from:

```rust
sw_galaxy_map_core::provision::arcgis
```

---

## Architecture

```text
ArcGIS FeatureServer
        │
        ▼
sw_galaxy_map_core::provision::arcgis
        │
        ▼
sw_galaxy_map_sync source adapter
        │
        ▼
ArcGIS classifier
   ├── known planets   → planets
   └── unnamed records → planets_unknown
```

---

## Current Features

- Download planet records from ArcGIS.
- Normalize ArcGIS attributes into a stable internal model.
- Export normalized ArcGIS data to JSON.
- Split ArcGIS records into:
  - known planets,
  - unknown unnamed ArcGIS records.
- Import/upsert known records into `planets`.
- Import/upsert unnamed records into `planets_unknown`.
- Auto-create `planets_unknown` from the `planets` column layout when missing.
- Support `--dry-run` for safe validation.
- Keep SQLite handled through `rusqlite`.
- Avoid `sqlx` for now to prevent workspace-level `libsqlite3-sys` conflicts.

---

## Unknown ArcGIS Records

ArcGIS sometimes contains records without a usable planet name.

These records are not discarded.

Instead, they are preserved in:

```text
planets_unknown
```

The `planets_unknown` table is created automatically if missing, using the same column names and SQLite column types as the configured `planets` table.

Records are stored with:

- original ArcGIS `FID`,
- empty `Planet`,
- empty `planet_norm`,
- available region/sector/system/grid data,
- available X/Y coordinates,
- ArcGIS hash,
- source flags.

This allows later inspection, review and promotion without losing source data.

---

## Coordinate Policy

ArcGIS X/Y values are currently treated as the canonical stored coordinates.

At this stage they are stored as parsecs, matching the ArcGIS source.

Values are rounded to 3 decimal places before DB insertion/update.

Recommended long-term policy:

```text
DB canonical coordinates: parsec
UI/export coordinates   : light years, when needed
conversion              : ly = pc * 3.26156
```

---

## Commands

### Show help

```bash
cargo run -p sw_galaxy_map_sync -- --help
```

### Show ArcGIS commands

```bash
cargo run -p sw_galaxy_map_sync -- arcgis --help
```

---

## ArcGIS: Fetch

Download ArcGIS data and save normalized records as JSON.

```bash
cargo run -p sw_galaxy_map_sync -- arcgis fetch --out res/arcgis_planets.json
```

Optional page size:

```bash
cargo run -p sw_galaxy_map_sync -- arcgis fetch \
  --page-size 2000 \
  --out res/arcgis_planets.json
```

Output format:

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
      "arcgis_hash": "...",
      "canon": 1,
      "legends": 0,
      "raw": {}
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
      "arcgis_hash": "...",
      "canon": 1,
      "legends": 0,
      "raw": {}
    }
  ]
}
```

---

## ArcGIS: Import into SQLite

Download from ArcGIS and upsert into SQLite:

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import \
  --db res/sw_planets.sqlite
```

Use custom tables:

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

## SQLite Expectations

The import command expects a `planets`-like table with at least these columns:

```sql
FID INTEGER,
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
Legends INTEGER
```

The upsert is based on `FID`.

The implementation intentionally uses manual `SELECT` + `INSERT`/`UPDATE` instead of `ON CONFLICT`, so `planets_unknown` does not require a primary-key constraint.

---

## Why MySQL is not included yet

MySQL is part of the future Astronavis direction, but version `0.3.0` intentionally avoids adding `sqlx`.

Reason:

- the workspace already uses `rusqlite`,
- enabling `sqlx/sqlite` can pull another incompatible `libsqlite3-sys`,
- the sync model must be stabilized before adding remote DB backends.

Recommended roadmap:

```text
v0.3.0  ArcGIS-first SQLite ingestion
v0.3.x  CSV merge on top of ArcGIS baseline
v0.4.0  Repository abstraction
v0.5.0  MySQL backend
```

---

## Suggested Tests

### 1. Build

```bash
cargo build -p sw_galaxy_map_sync
```

### 2. Format

```bash
cargo fmt --all -- --check
```

### 3. Clippy

```bash
cargo clippy -p sw_galaxy_map_sync --all-targets --all-features -- -D warnings
```

### 4. Unit tests

```bash
cargo test -p sw_galaxy_map_sync
```

### 5. Fetch ArcGIS JSON

```bash
cargo run -p sw_galaxy_map_sync -- arcgis fetch --out res/arcgis_planets.json
```

Verify:

- the JSON file exists,
- it contains `known` and `unknown` arrays,
- unnamed ArcGIS records are in `unknown`.

### 6. Dry-run import

```bash
cargo run -p sw_galaxy_map_sync -- arcgis import --db res/sw_planets.sqlite --dry-run
```

Expected behavior:

- fetches ArcGIS data,
- normalizes records,
- prints known/unknown statistics,
- does not modify the DB.

### 7. Real import on a copy

```powershell
Copy-Item .\res\sw_planets.sqlite .\res\sw_planets_arcgis_test.sqlite

cargo run -p sw_galaxy_map_sync -- arcgis import `
  --db res/sw_planets_arcgis_test.sqlite
```

Verify:

- known records are inserted/updated in `planets`,
- unnamed records are inserted/updated in `planets_unknown`,
- `planets_unknown` is created if missing,
- `arcgis_hash` changes only when source data changes,
- `deleted = 0` for imported records.

---

## License

Licensed under either:

- MIT License
- Apache License 2.0

at your option.
