# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.2.0] – 2026-01-15

### Added

- Automatic local database initialization on first use (`search`, `info`, `near`) if the database is missing.
- `db status` command to inspect local database path, metadata, counts, schema and FTS status.
- Full-Text Search (FTS5) support with automatic detection and fallback to indexed LIKE search.
- Normalized search table (`planet_search`) and FTS-backed search (`planets_fts`) when available.
- Alias-based planet lookup (name0/name1/name2).

### Improved

- Database provisioning moved to OS local application data directory.
- Search relevance and performance improved via FTS5 (`bm25`) when supported.
- CLI UX improvements for `db init`:
    - interactive overwrite confirmation when the database already exists
    - `--force` to bypass confirmation.
- Robust handling of invalid source records (missing Planet or X/Y).

### Fixed

- Dependency compatibility with `reqwest 0.13` using `rustls-tls-webpki-roots`.
- Clippy warnings and type-complexity issues resolved.

### Notes

- Some source records are intentionally skipped during import if required fields are missing.
- FTS5 availability depends on the SQLite build; fallback search is always available.

---

## [0.1.0] - 2026-01-15

### Added

- Initial release of the **sw_galaxy_map** CLI application.
- SQLite-backed local database for offline planet queries.
- Text-based planet search using normalized names and aliases.
- Planet detail command displaying all available information.
- Nearby planet search within a given radius using Euclidean distance
  on X/Y coordinates expressed in parsecs.
- Support for alias-based lookup derived from multiple known planet names.
- Clear attribution and acknowledgements for the original data source
  (Star Wars Galaxy Map by Henry Bernberg: read the [README](README.md) for further information).

### Notes

- This is the first public version of the project and should be considered
  an initial, evolving release.
- The database is intended for local, educational, and non-commercial use.
