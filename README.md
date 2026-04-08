# sw_galaxy_map

[![Crates.io](https://img.shields.io/crates/v/sw_galaxy_map_cli.svg)](https://crates.io/crates/sw_galaxy_map_cli)
[![Downloads](https://img.shields.io/crates/d/sw_galaxy_map_cli.svg)](https://crates.io/crates/sw_galaxy_map_cli)
[![Docs.rs](https://docs.rs/sw_galaxy_map_core/badge.svg)](https://docs.rs/sw_galaxy_map_core)
[![License](https://img.shields.io/crates/l/sw_galaxy_map_cli.svg)](https://github.com/umpire274/sw_galaxy_map)
[![Rust](https://github.com/umpire274/sw_galaxy_map/actions/workflows/rust.yml/badge.svg)](https://github.com/umpire274/sw_galaxy_map/actions)

---

## 🌌 Overview

**sw_galaxy_map** is a Rust workspace for exploring and maintaining the Star Wars galaxy using a local SQLite database, with:

* 🖥️ **CLI interface**
* 🧭 **Interactive TUI (ratatui)**
* 🪟 **GUI (egui/eframe)**
* ✏️ **Dedicated editing tool for data curation**

The application works **fully offline** once the database is initialized.

---

## ✨ Features

* 🔎 Planet search (exact + fuzzy)
* 🎯 Advanced filtering (region, sector, grid, status, canon/legends)
* 📍 Nearby planet search (Euclidean distance in parsecs)
* 🧪 Unknown planets workflow (staging + review)
* 🧭 Hyperspace routing engine with obstacle avoidance
* 📊 Galaxy statistics and analytics
* 🖥️ Interactive TUI with panels and navigation
* 📦 CSV export and JSON output support
* ✏️ Manual data editing with audit tracking

---

## 🧱 Workspace layout (0.15.x)

The project is organized as a Cargo workspace:

* `sw_galaxy_map_core` — domain logic, routing engine, database
* `sw_galaxy_map_cli` — CLI + TUI interface
* `sw_galaxy_map_gui` — graphical interface (egui)
* `sw_galaxy_map_sync` — CSV synchronization engine
* `sw_galaxy_map_edit` — command-line editor for manual data curation

---

## ✏️ Editing tool

The `sw_galaxy_map_edit` crate provides a dedicated tool for maintaining planet data.

### Features

* interactive editing wizard
* non-interactive updates via CLI
* field-level validation
* change preview
* audit trail (`entity_edit_log`)
* history inspection

### Example usage

```bash
cargo run -p sw_galaxy_map_edit
```

Interactive session:

```text
Field to edit: region
New value: Unknown Regions
Apply this change? yes
```

Non-interactive update:

```bash
cargo run -p sw_galaxy_map_edit -- set \
  --fid 1970 \
  --field x \
  --value 1834.100 \
  --reason "coordinate update"
```

---

## 🚀 Getting started

### Run CLI

```bash
cargo run -p sw_galaxy_map_cli -- <command>
```

### Run interactive TUI

```bash
cargo run -p sw_galaxy_map_cli
```

### Run GUI

```bash
cargo run -p sw_galaxy_map_gui
```

### Run editor

```bash
cargo run -p sw_galaxy_map_edit
```

---

## 🔎 Planet search

### Basic search

```bash
sw_galaxy_map search tatooine
```

### Fuzzy search (typo-tolerant)

```bash
sw_galaxy_map search tatoine --fuzzy
```

### Advanced filters

```bash
sw_galaxy_map search tatooine \
  --region "outer rim" \
  --sector "arkanis" \
  --grid "R-16" \
  --status active \
  --canon
```

---

## 🧭 Routing engine

### Compute route

```bash
sw_galaxy_map route compute tatooine dathomir
```

### Show route

```bash
sw_galaxy_map route show <id>
```

### Explain route (advanced)

```bash
sw_galaxy_map route explain <id>
```

Includes:

* ETA breakdown
* waypoint distances
* detour analysis
* routing diagnostics

---

## 📊 Galaxy statistics

```bash
sw_galaxy_map db stats --top 10
```

---

## 🧪 Unknown planets workflow

### List unknown planets

```bash
sw_galaxy_map unknown list
```

### Search nearby known planets

```bash
sw_galaxy_map unknown search <id> --near 1500
```

---

## 🗄️ Database lifecycle

### Initialize database

```bash
sw_galaxy_map db init
```

### Update database

```bash
sw_galaxy_map db update
```

### Rebuild search indexes

```bash
sw_galaxy_map db rebuild-search
```

---

## 🔄 Sync (CSV import)

```bash
sw_galaxy_map db sync --csv data.csv
```

---

## 🖥️ TUI (Interactive Mode)

The TUI provides a full interactive experience:

* 📜 Log panel (left)
* 🌍 Planet panels (right)
* 🧭 Navigation panel
* ⌨️ Command input with history

---

## 🧠 Routing model

The routing engine uses:

* Euclidean geometry (parsecs)
* planetary obstacles (circles)
* dynamic waypoint generation
* detour scoring system

---

## 🗃️ Persistence model

Main tables:

* `planets`
* `planets_unknown`
* `planet_aliases`
* `planet_search`
* `routes`
* `route_waypoints`
* `route_detours`
* `entity_edit_log`

---

## 📦 Installation

From crates.io:

```bash
cargo install sw_galaxy_map_cli
```

Editor:

```bash
cargo install sw_galaxy_map_edit
```

---

## ⚠️ Notes

* This project is intended for **educational and non-commercial use**
* Works fully offline after database initialization
* Requires SQLite (bundled via rusqlite)

---

## 🙏 Acknowledgements

Planetary data derived from:

* http://www.swgalaxymap.com/ — **Henry Bernberg**
* https://www.starwars.com/star-wars-galaxy-map

This project is for **educational and non-commercial use only** and is not affiliated with Lucasfilm or Disney.
