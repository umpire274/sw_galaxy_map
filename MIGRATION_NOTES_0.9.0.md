# sw_galaxy_map 0.9.0 migration notes

This is phase 1 of the workspace migration.

## What changed

- Created a Cargo workspace with three crates:
  - `sw_galaxy_core`
  - `sw_galaxy_cli`
  - `sw_galaxy_gui`
- Moved the previous `src/` tree into `crates/sw_galaxy_core/src/`
- Added thin binary entrypoints for CLI and GUI
- Centralized common dependency versions at workspace root

## Current status

This is an architectural bootstrap, not the final split yet.
The `sw_galaxy_core` crate still temporarily contains `cli` and `gui` modules so the project can be migrated incrementally.

## Next steps

1. Remove `db -> cli` dependencies
2. Remove terminal output from `db` modules
3. Move CLI-only modules from core into `sw_galaxy_cli`
4. Move GUI-only modules from core into `sw_galaxy_gui`
5. Introduce pure domain/service APIs in `sw_galaxy_core`


## Migration 2

This step starts the boundary cleanup required by the workspace split:

- introduced `sw_galaxy_map_core::domain::RouteListSort`
- removed the direct `db -> cli` dependency for route list sorting
- kept the CLI using the same `clap` value enum through the shared domain type

At this stage, the codebase is still intentionally transitional: CLI and GUI modules remain inside the core crate bootstrap, but the first domain type now lives outside the CLI layer.


## migration4
- Moved CLI sources from `sw_galaxy_map_core` into `sw_galaxy_map_cli`.
- Moved shared input validation into `sw_galaxy_map_core::validate`.
- Removed terminal UI code from the core crate.


## migration5

- moved the GUI implementation out of `sw_galaxy_map_core` into `sw_galaxy_map_gui`
- `sw_galaxy_map_gui` now owns the egui/eframe application code and GUI-specific dependencies
- the GUI launches the sibling `sw_galaxy_map_cli` executable instead of recursively spawning itself
- `sw_galaxy_map_core` no longer exports a `gui` module


## migration5.1

- `cargo run -p sw_galaxy_map_cli` now launches only the CLI.
- Running the CLI crate without subcommands now starts the interactive CLI instead of delegating to the GUI.
- The legacy `--cli` discriminant has been removed from the CLI crate.
- `cargo run -p sw_galaxy_map_gui` continues to launch only the GUI.


## migration5.2

- cleaned up the remaining split-frontends messaging after the workspace refactor
- `sw_galaxy_map_cli` now documents itself as CLI-only and no longer mentions mixed CLI/GUI startup
- `sw_galaxy_map_gui` comments and GUI labels now describe launching the sibling CLI executable, not the current executable
- README updated to document the explicit workspace entrypoints for CLI and GUI
