# sw_galaxy_map_gui

`sw_galaxy_map_gui` is the graphical frontend for the **sw_galaxy_map** project, built with `egui` / `eframe`.

It provides a desktop interface for exploring the Star Wars galaxy using the same local SQLite database shared by the CLI.

## Features

* graphical planet search
* planet detail display
* route exploration
* integration with the shared `sw_galaxy_map_core` logic
* local/offline database usage

## Run

From the workspace:

```bash
cargo run -p sw_galaxy_map_gui
```

## Scope

This crate is the GUI layer of the project.

The shared logic for:

* database access
* search
* unknown planets
* routing
* persistence

is implemented in `sw_galaxy_map_core`.

## Notes

* The GUI and CLI are separate frontends built on the same core.
* The Windows executable embeds its application icon through the GUI crate build pipeline.

## Installation

After publishing to crates.io:

```bash
cargo install sw_galaxy_map_gui
```

## License

Licensed under either:

* MIT
* Apache-2.0

## Acknowledgements

Planetary data are based on the Star Wars Galaxy Map by Henry Bernberg.

This project is for educational and non-commercial use only.
