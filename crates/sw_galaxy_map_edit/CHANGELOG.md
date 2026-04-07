# Changelog

All notable changes to this crate will be documented in this file.

## [0.1.0] - 2026-04-07

### Added

* Initial standalone crate `sw_galaxy_map_edit` for manual data curation.
* Command-line interface with subcommands:

    * `find` — search planets by name, alias, or FID.
    * `edit` — interactive editing mode (wizard).
    * `set` — non-interactive single-field update.
    * `history` — view audit history for a specific entity.
    * `fields` — list editable fields and their metadata.

### Interactive Editing

* Guided wizard for:

    * resolving a planet (by name, alias, or FID)
    * selecting a field to edit
    * entering a new value
    * previewing changes
    * confirming updates

### Non-interactive Editing

* `set` command for scripted updates:

    * supports both `--fid` and `--planet`
    * supports `--yes` for non-interactive execution
    * integrates validation and audit logging

### Validation

* Field-level validation system with:

    * blocking errors (e.g. invalid grid format, invalid numeric values)
    * non-blocking warnings (e.g. whitespace, suspicious values)
* Basic validation rules for:

    * grid format (`L-9`, `AA-12`, etc.)
    * numeric fields (`x`, `y`, `lat`, `long`)
    * text normalization warnings

### Audit System

* Introduced generic audit table `entity_edit_log`:

    * supports multiple entity types (not only planets)
    * stores field-level changes
    * includes timestamp, reason, and source
* Atomic update + audit logging via transaction

### History Inspection

* `history` command:

    * shows recent changes for a given entity
    * supports filtering by FID or name
    * configurable limit

### Developer Experience

* Modular architecture:

    * `edit/` for editing primitives
    * `validate/` for validation logic
    * `audit/` for logging and history
    * `resolve/` for entity lookup
    * `output/` for formatting
* Strong separation between:

    * parsing
    * validation
    * persistence
    * presentation

### Notes

* Spatial validation (grid ↔ coordinates consistency) intentionally deferred.
* Future improvements may include:

    * batch updates
    * advanced validation rules
    * diff/inspection tools
    * metadata-driven field definitions
