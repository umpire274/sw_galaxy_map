# sw_galaxy_map

[![Crates.io](https://img.shields.io/crates/v/sw_galaxy_map_cli.svg)](https://crates.io/crates/sw_galaxy_map_cli)
[![Downloads](https://img.shields.io/crates/d/sw_galaxy_map_cli.svg)](https://crates.io/crates/sw_galaxy_map_cli)
[![Docs.rs](https://docs.rs/sw_galaxy_map_core/badge.svg)](https://docs.rs/sw_galaxy_map_core)
[![License](https://img.shields.io/crates/l/sw_galaxy_map_cli.svg)](https://github.com/umpire274/sw_galaxy_map)
[![Rust](https://github.com/umpire274/sw_galaxy_map/actions/workflows/rust.yml/badge.svg)](https://github.com/umpire274/sw_galaxy_map/actions)

**sw_galaxy_map** is a Rust workspace for exploring the Star Wars galaxy using a local SQLite database, with both CLI
and GUI frontends.

The application provides tools to:

* search for planets by name or alias,
* display detailed information about a planet,
* find nearby planets using Euclidean distance on X/Y coordinates (parsecs),
* inspect and manage **unknown (unclassified) planets**,
* compute hyperspace routes avoiding planetary obstacles.

The project is designed to work **offline** once the database is available and is intended for educational and
non-commercial use.

---

## Workspace layout (0.9.x)

The project is organized as a Cargo workspace:

* `sw_galaxy_map_core` — domain logic, routing engine, SQLite access
* `sw_galaxy_map_cli` — command-line interface
* `sw_galaxy_map_gui` — egui/eframe graphical interface

### Run CLI

```bash
cargo run -p sw_galaxy_map_cli
```

### Run GUI

```bash
cargo run -p sw_galaxy_map_gui
```

---

# 🔎 Planet search

Search planets by name or alias:

```bash
sw_galaxy_map search tatooine
```

Output includes **coordinates (X/Y)**:

```text
FID      Planet            Region        Sector         System         Grid     X        Y
--------------------------------------------------------------------------------------------
1234     Tatooine          Outer Rim     Arkanis        Tatoo          R-16     1040.12  -333.45
```

---

# 🧪 Unknown planets workflow

## Overview

Starting from **v0.9.6**, `planets_unknown` is no longer a simple dump of skipped rows, but a **staging table** aligned
with `planets`.

It is used to:

* inspect incomplete or malformed data
* manually review and classify entries
* support future editing/promoting workflows

## Table features

* internal `id` (stable primary key)
* nullable `fid`, `x`, `y`
* normalized name (`planet_norm`)
* structural fields (region, sector, system, etc.)
* workflow flags:

    * `reviewed`
    * `promoted`
* `reason` describing why the row was skipped
* `notes` for manual annotations

---

## Commands

### List unknown planets

```bash
sw_galaxy_map unknown list
sw_galaxy_map unknown list --page 2
sw_galaxy_map unknown list --page 2 --page-size 50
```

Features:

* pagination
* stable internal IDs
* safe handling of NULL values

---

### Search nearby known planets from an unknown

```bash
sw_galaxy_map unknown search <id> --near 1500
```

Uses unknown coordinates → finds nearby known planets.

Fails gracefully if coordinates are missing.

---

### NEW (v0.9.10): Find unknown near a known planet

```bash
sw_galaxy_map unknown near tatooine --range 1500
```

Finds unknown planets within a radius from a known planet.

Features:

* uses `planets.X` / `planets.Y`
* excludes unknown entries without coordinates
* sorted by distance
* shows reason and workflow status

Example:

```text
Unknown planets near Tatooine (X=1040.12, Y=-333.45) within 1500 parsecs

#  12 | fid=9981   | Unknown System Alpha | x=  999.23 | y= -210.88 | dist=145.77 | reason=missing_region
#  43 | fid=-      | (unknown)            | x= 1102.01 | y= -412.09 | dist= 98.43 | reason=missing_fid
```

---

# 🗄️ Database lifecycle

## Initialization

```bash
sw_galaxy_map db init
```

Now correctly:

* creates full schema (including routes & waypoints)
* populates both:

    * `planets`
    * `planets_unknown`

## Update

```bash
sw_galaxy_map db update
```

Features:

* incremental sync (no full rebuild)
* preserves `planets_unknown.id`
* stable CLI references

---

# 🧭 Routing engine

The routing engine computes hyperspace paths avoiding planetary obstacles.

## Key concepts

* planets = circular obstacles
* routes = polylines
* detours = dynamically generated waypoints

---

## Route command

```bash
sw_galaxy_map route compute tatooine dathomir
```

---

# 🧷 Waypoints & fingerprinting

Computed waypoints are deduplicated using a **fingerprint**:

* ensures reuse across routes
* prevents duplicate inserts
* enables caching

---

# 🗃️ Persistence model

Main tables:

* `planets`
* `planets_unknown`
* `waypoints`
* `routes`
* `route_waypoints`
* `route_detours`

---

## Acknowledgements

The planetary data used by this project were obtained from the **Star Wars Galaxy Map**:

[http://www.swgalaxymap.com/](http://www.swgalaxymap.com/)

Created and maintained by **Henry Bernberg**.

This project is for **educational and non-commercial use only** and is not affiliated with Lucasfilm or Disney.
