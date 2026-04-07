# sw_galaxy_map_edit

[![Crates.io](https://img.shields.io/crates/v/sw_galaxy_map_edit.svg)](https://crates.io/crates/sw_galaxy_map_edit)
[![License](https://img.shields.io/crates/l/sw_galaxy_map_edit.svg)](https://github.com/umpire274/sw_galaxy_map)

Command-line editor for maintaining planet records in the local `sw_galaxy_map` SQLite database.

This crate provides a dedicated data curation tool for updating planet metadata, with validation, audit logging, and history tracking.

---

## Features

* 🔎 Find planets by name, alias, or FID
* ✏️ Interactive editing wizard
* ⚡ Non-interactive updates via CLI (`set` command)
* 🧪 Field-level validation (errors and warnings)
* 📝 Audit trail for all modifications
* 📜 History inspection for previous edits
* 📋 Built-in field reference (`fields` command)

---

## Installation

```bash
cargo install sw_galaxy_map_edit
```

---

## Usage

### Find a planet

```bash
sw_galaxy_map_edit find tatooine
sw_galaxy_map_edit find 1970
```

---

### Interactive editing

```bash
sw_galaxy_map_edit
```

Starts a guided wizard to:

* select a planet
* choose a field
* preview changes
* confirm update

---

### Non-interactive update

```bash
sw_galaxy_map_edit set \
  --fid 1970 \
  --field x \
  --value 1834.100 \
  --reason "coordinate update"
```

Skip confirmation:

```bash
sw_galaxy_map_edit set ... --yes
```

---

### View edit history

```bash
sw_galaxy_map_edit history --fid 1970
```

---

### List editable fields

```bash
sw_galaxy_map_edit fields
```

---

## Validation

The editor performs field-level validation:

* ❌ Errors (blocking)

    * invalid grid format
    * invalid numeric values
* ⚠️ Warnings (non-blocking)

    * suspicious values
    * whitespace issues

---

## Audit system

All changes are recorded in the `entity_edit_log` table:

* entity type and ID
* field name
* old and new values
* timestamp
* optional reason and source

---

## Notes

* The tool operates on a local SQLite database initialized via `sw_galaxy_map_core`.
* Spatial validation (grid ↔ coordinates consistency) is intentionally limited and may evolve in future versions.

---

## License

MIT OR Apache-2.0
