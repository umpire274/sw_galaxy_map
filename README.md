# sw_galaxy_map

[![CI](https://img.shields.io/github/actions/workflow/status/umpire274/sw_galaxy_map/rust.yml?branch=main\&label=CI)](https://github.com/umpire274/sw_galaxy_map/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](./LICENSE-MIT)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue)](./LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/Rust-Edition%202024-orange)](./Cargo.toml)
[![Workspace](https://img.shields.io/badge/Cargo-Workspace-informational)](./Cargo.toml)
[![Status](https://img.shields.io/badge/status-0.9.0%20migration-success)](./CHANGELOG.md)

**sw_galaxy_map** is an offline Star Wars galaxy explorer written in Rust.

It provides a local SQLite-backed navigation and lookup system for planets, aliases, routes, and computed detour waypoints across a 2D galactic map expressed in parsecs. The project is intended for educational and non-commercial use.

Starting with **0.9.0**, the codebase is organized as a **Cargo workspace** with dedicated crates for shared logic, the command-line interface, and the graphical interface. This separation makes the project easier to evolve, test, and maintain. The current README already reflects that three-crate workspace split and explicit frontend launch behavior, which this rewritten version preserves and expands. fileciteturn4file0L1-L31

---

## Features

* Search planets by name or alias
* Display detailed information for a single planet
* Find nearby planets within a configurable radius
* Compute hyperspace routes between planets
* Automatically generate safe detour waypoints around planetary no-fly zones
* Persist routes, route polylines, detour decisions, and reusable computed waypoints in SQLite
* Use the project entirely offline once the local database is available
* Choose between a **CLI frontend** and a **GUI frontend**

---

## Workspace layout

Starting with the `0.9.0` migration, the project is structured as a Cargo workspace with three crates: `sw_galaxy_map_core` for shared logic and persistence, `sw_galaxy_map_cli` for the command-line interface and interactive shell, and `sw_galaxy_map_gui` for the egui/eframe graphical interface. fileciteturn4file0L14-L31

```text
sw_galaxy_map/
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── crates/
│   ├── sw_galaxy_map_core/
│   ├── sw_galaxy_map_cli/
│   └── sw_galaxy_map_gui/
└── .github/
    └── workflows/
```

### Crates

#### `sw_galaxy_map_core`

Shared domain logic:

* SQLite access and migrations
* data models
* routing engine
* detour generation
* route persistence
* validation helpers
* common utilities

#### `sw_galaxy_map_cli`

Terminal frontend:

* subcommands
* interactive shell
* terminal rendering
* export helpers
* script-friendly workflows

#### `sw_galaxy_map_gui`

Graphical frontend:

* egui/eframe application
* graphical command launcher
* route and planet exploration UI
* integration with the CLI sibling executable where needed

---

## Running the project

### Run the CLI

```bash
cargo run -p sw_galaxy_map_cli
```

This starts **only** the CLI frontend.

* If you pass a subcommand, it runs a one-shot command.
* If you launch it without subcommands, it starts the interactive shell.

Examples:

```bash
cargo run -p sw_galaxy_map_cli -- search tatooine
cargo run -p sw_galaxy_map_cli -- info coruscant
cargo run -p sw_galaxy_map_cli -- near alderaan --radius 25
```

### Run the GUI

```bash
cargo run -p sw_galaxy_map_gui
```

This starts **only** the GUI frontend.

The two frontends are intentionally separated:

* `cargo run -p sw_galaxy_map_cli` launches only the CLI
* `cargo run -p sw_galaxy_map_gui` launches only the GUI

That explicit split is already part of the current project direction and should remain the expected behavior for `0.9.0`. fileciteturn4file0L20-L31

---

## Development commands

### Format

```bash
cargo fmt --all
```

### Check

```bash
cargo check --workspace --all-targets
```

### Clippy

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Test

```bash
cargo test --workspace
```

### Full local validation sequence

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## Continuous Integration

The repository uses a GitHub Actions workflow that checks formatting, performs `cargo check`, runs Clippy with warnings denied, and executes workspace tests. This aligns well with the new workspace organization and is a natural evolution of the current CI setup. The previous workflow already performed formatting, Clippy, and tests, while the updated direction adds `cargo check` and uses workspace-oriented commands. fileciteturn4file5L1-L26

---

## Routing engine overview

The routing engine computes hyperspace routes on a 2D galactic map using X/Y coordinates expressed in parsecs. The ideal route is a straight line between origin and destination, but planets create **hyperspace no-fly zones** that the route cannot cross. When the direct line intersects one or more obstacles, the engine inserts detour waypoints to bypass them safely. This general model, including collision detection and waypoint generation, is already documented in the existing README and remains one of the project’s most distinctive technical features. fileciteturn4file1L13-L64

### Obstacle model

Each planet is treated as a circular obstacle in the galactic plane.

The obstacle radius is controlled by the route safety configuration and represents navigational constraints such as:

* gravitational mass shadows
* hyperspace shear
* interdiction fields
* standard astrogation safety margins

This radius does **not** represent the planet’s physical diameter.

### Collision detection

For each route segment, the engine:

1. computes the closest point on the segment to nearby planets
2. checks whether the distance is below the configured safety radius
3. resolves the earliest hard collision first

### Detour candidate generation

When a collision is found, the engine generates candidate bypass points around the obstacle using multiple directions, including:

* radial
* lateral
* forward
* backward
* diagonal directions

This improves robustness in dense or complex regions of the map. The current README already describes this multi-direction candidate generation and offset growth strategy in detail. fileciteturn4file1L31-L64

### Candidate scoring

Each valid detour candidate is evaluated using a weighted score that combines:

* path length increase
* turn penalty
* backtracking penalty
* proximity penalty to nearby obstacles

The candidate with the lowest total score is selected.

### Iterative refinement

After choosing a detour waypoint, the engine inserts it into the route polyline and restarts collision analysis from the beginning. The algorithm stops when:

* no collisions remain, or
* the configured maximum iteration count is reached

The final route is therefore explainable, stable, and suitable for persistence, debugging, and future visualization. fileciteturn4file2L1-L37

---

## SQLite persistence model

The project persists computed routes and generated detour waypoints in a local SQLite database. The current README already documents this as a caching and inspection layer for route computation, route replay, and future visualization support. fileciteturn4file3L87-L101

### Why persistence matters

Persistence supports:

* route caching
* easier debugging and inspection
* deterministic `route show` / `route last` style workflows
* future analytics and map visualization
* progressive accumulation of reusable navigation waypoints

### Main tables

#### `waypoints`

Global waypoint catalog containing both manual and computed waypoints.

#### `waypoint_planets`

Association table linking waypoints to nearby or related planets.

#### `routes`

Route cache entries keyed by origin/destination with associated options, length, iterations, status, and timestamps.

The current README already emphasizes a key design rule: routes are unique per origin/destination pair, with recomputation updating the existing record rather than creating duplicates. fileciteturn4file4L11-L39

#### `route_waypoints`

Ordered route polyline entries for each stored route.

The current persistence description also documents a replace strategy where recomputation removes and reinserts the stored polyline for the route. fileciteturn4file4L40-L66

#### Additional route detail tables

Depending on the current schema version, the database may also include tables for detour decisions, score breakdowns, and other route-related metadata used for inspection and debugging.

---

## Typical use cases

### Quick exploration from the terminal

Use the CLI when you want fast, scriptable queries:

* search planets
* inspect aliases
* compute routes
* review cached routes
* export route data

### Interactive usage

Use the CLI interactive shell when you want a terminal-first exploratory workflow without retyping the executable name for every command.

### Graphical exploration

Use the GUI when you want a more visual workflow for searching planets, browsing information, and launching route-related actions from a desktop interface.

---

## Data source and acknowledgements

The planetary data used by this project come from the **Star Wars Galaxy Map** project maintained by **Henry Bernberg**. The existing README already credits that source and states that this project uses the data for educational and non-commercial purposes only, with no affiliation with Lucasfilm or Disney. fileciteturn4file0L33-L46

Original project:

* **Star Wars Galaxy Map** — Explore the Galaxy Far, Far Away

If you find that dataset valuable, please support the original author through the official channels listed in the upstream project.

---

## License

This project is dual-licensed under either of the following, at your option:

* MIT license
* Apache License 2.0

See:

* `LICENSE-MIT`
* `LICENSE-APACHE`

---

## Status

The `0.9.0` series focuses on the workspace migration and architectural cleanup:

* shared logic consolidated in `sw_galaxy_map_core`
* CLI and GUI separated into their own crates
* frontend launch behavior made explicit
* CI aligned with workspace-based development

This makes the project easier to maintain and creates a stronger base for future work on routing, persistence, visualization, and packaging.

---

## Contributing

Contributions, issue reports, and suggestions are welcome.

When contributing locally, please make sure the project passes:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
