# sw_galaxy_map_cli

`sw_galaxy_map_cli` is the command-line frontend for the **sw_galaxy_map** project.

It provides an interactive and script-friendly interface to explore the Star Wars galaxy through a local SQLite database.

## Features

* search planets by name or alias
* display planet details
* find nearby planets
* inspect unknown / unclassified planets
* compute routes between planets
* initialize and update the local database

## Run

From the workspace:

```bash
cargo run -p sw_galaxy_map_cli
```

Once installed:

```bash
sw_galaxy_map
```

## Example commands

### Search planets

```bash
sw_galaxy_map search tatooine
```

### Find nearby planets

```bash
sw_galaxy_map near tatooine --range 1500
```

### List unknown planets

```bash
sw_galaxy_map unknown list
sw_galaxy_map unknown list --page 2
sw_galaxy_map unknown list --page 2 --page-size 50
```

### Search known planets near an unknown record

```bash
sw_galaxy_map unknown search 42 --near 1500
```

### Find unknown planets near a known planet

```bash
sw_galaxy_map unknown near tatooine --range 1500
```

### Compute a route

```bash
sw_galaxy_map route compute hapes dathomir
```

### Initialize or update the database

```bash
sw_galaxy_map db init
sw_galaxy_map db update
```

## Unknown planets support

The CLI supports inspection of incomplete or skipped records stored in `planets_unknown`.

Features include:

* paginated listing
* stable internal IDs
* nullable `fid`, `x`, `y` handling
* proximity queries from unknown → known
* proximity queries from known → unknown

## Installation

After publishing to crates.io:

```bash
cargo install sw_galaxy_map_cli
```

## License

Licensed under either:

* MIT
* Apache-2.0

## Acknowledgements

Planetary data are based on the Star Wars Galaxy Map by Henry Bernberg.

This project is for educational and non-commercial use only.
