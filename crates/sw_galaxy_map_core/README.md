# sw_galaxy_map_core

`sw_galaxy_map_core` is the core library for the **sw_galaxy_map** project.

It provides the shared logic used by both the CLI and GUI frontends, including:

* SQLite schema provisioning and migrations
* planet search and lookup
* nearby planet queries based on X/Y galactic coordinates (parsecs)
* unknown planet staging workflows
* route computation and waypoint persistence
* normalization and utility helpers

## Scope

This crate is intended for:

* shared internal use inside the `sw_galaxy_map` workspace
* future reuse in other Rust tools or frontends
* database-backed Star Wars galaxy exploration workflows

## Main capabilities

* search planets by name or alias
* retrieve detailed planet information
* compute nearby planets in parsec range
* inspect unknown / unclassified planets
* compute and persist routes with detour waypoints

## Database model

The core crate manages the SQLite persistence layer, including:

* `planets`
* `planets_unknown`
* `waypoints`
* `routes`
* `route_waypoints`
* `route_detours`

It also supports:

* schema creation
* migrations
* initial provisioning
* incremental updates

## Unknown planets workflow

The `planets_unknown` table is used as a staging area for skipped or incomplete records.

It supports:

* stable internal IDs
* nullable source fields such as `fid`, `x`, and `y`
* workflow flags like `reviewed` and `promoted`
* proximity queries between known and unknown planets

## Intended usage

This crate is primarily consumed by:

* `sw_galaxy_map_cli`
* `sw_galaxy_map_gui`

## License

Licensed under either:

* MIT
* Apache-2.0

## Acknowledgements

Planetary data are based on the Star Wars Galaxy Map by Henry Bernberg.

This project is for educational and non-commercial use only.
