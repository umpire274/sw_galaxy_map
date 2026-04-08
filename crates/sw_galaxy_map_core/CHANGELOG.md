## sw_galaxy_map_core

## [0.15.2] - 2026-04-08

### Added

* Added schema migration **v13**.
* Introduced new column `grid_unit` in:

    * `planets`
    * `planets_unknown`
    * Values:

        * `"pc"` (parsec, legacy data)
        * `"ly"` (light years, new standard)
* Added coordinate conversion utilities in `utils::normalize` to convert between parsecs and light years.

### Changed

* Updated coordinate system from **parsecs (pc)** to **light years (ly)** to align with the new official galaxy map.
* All existing `X` and `Y` coordinates are automatically converted from parsecs to light years during migration.
* Migrated stored coordinates from parsecs to light years while preserving raw precision in the database.
* Added separate raw and display coordinate conversion helpers so UI layers can present rounded values without affecting
  stored data.

### Removed

* Removed obsolete table `planets_official` (now fully integrated into `planets`).

### Fixed

* Ensured consistency of derived search structures after coordinate conversion by rebuilding `planet_search` data during
  migration.
* Improved migration robustness by handling nullable fields and ensuring idempotent schema updates.
* Fixed v13 migration to correctly handle NULL coordinates in `planets_unknown`, preventing migration failures on
  incomplete records.

### Migration Notes

* Migration v13 performs:

    * schema update (`grid_unit`)
    * data transformation (pc → ly)
    * cleanup (`planets_official`)
    * rebuild of search artifacts
* Existing databases are transparently upgraded at runtime.
* Coordinate conversion uses:

    * `1 parsec = 3.26156 light years`

### Notes

* Future work may introduce runtime support for dual-unit handling (pc/ly) in CLI and routing logic.

---

### [0.15.1] - 2026-04-03

(no changes)

---

### [0.15.0] - 2026-04-03

### ✨ Added

- New `GalaxyStats` struct in `sw_galaxy_map_core::model`.
- New `galaxy_stats()` query function with aggregate SQL queries.
- New `FuzzyHit` struct and `fuzzy` module in `sw_galaxy_map_core::utils`.

### 🔄 Changed

- `SearchFilter` struct now includes `fuzzy: bool`.

### ♻️ Refactor

- Split `db::queries` into domain modules (`planets`, `routes`, `waypoints`, `stats`, etc.).
- Introduced `row_mappers.rs` for centralized row → model mapping.
- Removed monolithic `queries/mod.rs` logic.
- Cleaned unused constants and imports.
- Preserved public API (no breaking changes).

---

### [0.14.0] - 2026-04-02

### ✨ Added

- New `SearchFilter` struct in `sw_galaxy_map_core::model`.
- New `search_planets_filtered()` query function with dynamic SQL construction.

### 🔄 Changed

- `PlanetSearchRow` now includes `status: Option<String>`.

### 🧠 Internal

- `validate_search()` refactored to accept a single `&SearchFilter`.
- Revival logic and update helpers aligned to `status`.

---

### [0.13.0] - 2026-04-01

### ✨ Added

- Added `db rebuild-search` support at the data layer.
- Introduced **hyperspace ETA estimation engine** (`routing::eta`, `routing::hyperspace`).
- Added **sublight travel time estimation** (`routing::sublight`).
- Added `planets.status` field support across the data layer.
- Added `seed_planets_official()` provisioning function.

### 🧠 Internal

- New `hyperspace`, `sublight`, and `eta` modules.
- New `rebuild_search_indexes()` public entry point.
- Routing module reorganized.

### 🐛 Fixed

- Search queries now correctly filter out soft-deleted/skipped/invalid planets via `status`.

---

### [0.12.0] - 2026-03-27

(no direct core-specific changes separated in the original changelog)

---

### [0.11.0] - 2026-03-26

(no direct core-specific changes separated in the original changelog)

---

### [0.9.10] - 2026-03-21

### Added

- Added core query helpers for nearby unknown planets.

---

### [0.9.9] - 2026-03-21

### 🐛 Fixed

- Fixed schema/runtime mismatches affecting route and unknown-planet support.
- Improved alignment between models and DB schema.

### 🧩 Internal

- Reduced type complexity in provisioning.
- Improved consistency between provisioning, migrations, and runtime schema.

---

### [0.9.7] - 2026-03-20

### 🐛 Fixed

- Updated `UnknownPlanet.fid` to `Option<i64>`.
- Ensured consistent handling of nullable fields across the DB layer.

---

### [0.9.6] - 2026-03-20

### Added

- Expanded `planets_unknown` schema for staging/edit workflows.
- Added workflow fields (`reviewed`, `promoted`, `notes`) and normalized storage.

---

### [0.9.5] - 2026-03-20

### Added

- Added core query helpers for listing/searching unknown planets.

---

### [0.9.1] - 2026-03-19

(no direct core-specific changes separated in the original changelog)

---

### [0.9.0] - Unreleased

### Changed

- Project reorganized as a Cargo workspace with `sw_galaxy_map_core` extracted as a dedicated crate.

---

### [0.5.3] - 2026-01-19

### ✨ Routing & Persistence

- Persistence of detour waypoints as `computed`.
- Deterministic fingerprint-based deduplication.
- Automatic waypoint → obstacle planet linking with role `avoid`.
- Full persistence of routes, route polyline, and detour decision details.

### 🧪 Tests

- Added routing integration tests.
- Shared helper `assert_collision_free`.

---

### [0.5.2] - 2026-01-19

### ✨ Routing & Persistence

- Introduced route persistence support and route-related schema up to v8.

### 🗄️ Database

- Added `routes`, `route_waypoints`, `route_detours`.

### 🧹 Internal

- Improved separation between routing logic and persistence.

---

### [0.5.1] - 2026-01-19

### 🚀 Routing Engine

- Implemented first working routing engine between two planets.
- Added obstacle-aware geometry and scoring model.

### 🧠 Collision Handling

- Unified collision handling and validation.

---

### [0.5.0] - 2026-01-16

### Added

- Introduced waypoint catalog and waypoint DB queries.

### Changed

- Moved DB-related sources under `src/db/`.

---

### [0.4.1] - 2026-01-16

### Changed

- Consolidated planet-related queries into `db/queries.rs`.
- Standardized column-name based row mapping.

### Added

- Added `Planet::info_planet_url()`.

### Fixed

- Fixed SQL alias drift and query closure result mismatches.

---

### [0.4.0] - 2026-01-16

### Added

- Incremental `db update` data-layer support with hashing and soft delete.

### Internal

- Shared normalization and hash computation reused across init/update.

---

### [0.2.0] - 2026-01-15

### Added

- Automatic local database initialization support.
- FTS/search data layer.
- Alias-based lookup support.

---

### [0.1.0] - 2026-01-15

### Added

- Initial database-backed planet/domain logic.
