pub mod aliases;
pub mod near;
pub mod planets;
pub mod routes;
mod search;
pub mod stats;
pub mod unknown;
pub mod waypoints;

mod row_mappers;

pub use aliases::*;
pub use near::*;
pub use planets::*;
pub use routes::*;
pub use search::*;
pub use stats::*;
pub use unknown::*;
pub use waypoints::*;

#[cfg(test)]
mod tests {
    use super::{
        UnknownPlanetUpdate, near_planets, near_planets_excluding_fid, search_planets,
        update_unknown_planet,
    };
    use rusqlite::Connection;

    fn setup_search_db() -> Connection {
        let con = Connection::open_in_memory().expect("in-memory sqlite");
        con.execute_batch(
            r#"
            CREATE TABLE planets (
                FID INTEGER PRIMARY KEY,
                Planet TEXT NOT NULL,
                Region TEXT,
                Sector TEXT,
                System TEXT,
                Grid TEXT,
                X REAL NOT NULL,
                Y REAL NOT NULL,
                deleted INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE planet_search (
                planet_fid INTEGER NOT NULL,
                search_norm TEXT NOT NULL
            );
            INSERT INTO planets (FID, Planet, Region, Sector, System, Grid, X, Y, deleted) VALUES
                (1, 'Alderaan', 'Core Worlds', 'Alderaan', 'Alderaan', 'L-4', 10.0, 10.0, 0),
                (2, 'Tatooine', 'Outer Rim', 'Arkanis', 'Tatoo', 'R-16', 20.0, 25.0, 0),
                (3, 'Deleted', 'Unknown', NULL, NULL, NULL, 50.0, 50.0, 1);
            INSERT INTO planet_search (planet_fid, search_norm) VALUES
                (1, 'alderaan house organa'),
                (2, 'tatooine luke skywalker'),
                (3, 'deleted hidden');
            "#,
        )
        .expect("schema setup");
        con
    }

    fn setup_unknown_db() -> Connection {
        let con = Connection::open_in_memory().expect("in-memory sqlite");
        con.execute_batch(
            r#"
            CREATE TABLE planets_unknown (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                fid         INTEGER,
                planet      TEXT NOT NULL,
                planet_norm TEXT NOT NULL,
                region      TEXT,
                sector      TEXT,
                system      TEXT,
                grid        TEXT,
                x           REAL,
                y           REAL,
                arcgis_hash TEXT,
                deleted     INTEGER NOT NULL DEFAULT 0,
                canon       INTEGER,
                legends     INTEGER,
                zm          INTEGER,
                name0       TEXT,
                name1       TEXT,
                name2       TEXT,
                lat         REAL,
                long        REAL,
                ref         TEXT,
                status      TEXT,
                cregion     TEXT,
                cregion_li  TEXT,
                reason      TEXT,
                reviewed    INTEGER NOT NULL DEFAULT 0,
                promoted    INTEGER NOT NULL DEFAULT 0,
                notes       TEXT
            );
            INSERT INTO planets_unknown (
                id, fid, planet, planet_norm, region, sector, system, grid, x, y, canon,
                legends, cregion, cregion_li, reviewed, promoted, notes
            ) VALUES (
                7, 1007, 'TBD World', 'tbd world', 'Unknown Regions', 'Sector A',
                'System A', 'A-1', 1.0, 2.0, NULL, NULL, NULL, NULL, 0, 0, 'draft'
            );
            "#,
        )
        .expect("unknown schema setup");
        con
    }

    #[test]
    fn search_planets_ignores_empty_query_and_non_positive_limit() {
        let con = setup_search_db();

        assert!(
            search_planets(&con, "", 10)
                .expect("empty query")
                .is_empty()
        );
        assert!(
            search_planets(&con, "   ", 10)
                .expect("blank query")
                .is_empty()
        );
        assert!(
            search_planets(&con, "alderaan", 0)
                .expect("zero limit")
                .is_empty()
        );
    }

    #[test]
    fn near_planets_validates_inputs_and_filters_results() {
        let con = setup_search_db();

        let rows = near_planets(&con, 9.0, 9.0, 2.0, 10).expect("near query");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].planet, "Alderaan");

        assert!(near_planets(&con, 0.0, 0.0, -1.0, 10).is_err());
        assert!(near_planets(&con, f64::NAN, 0.0, 1.0, 10).is_err());
        assert!(
            near_planets(&con, 0.0, 0.0, 1.0, 0)
                .expect("zero limit")
                .is_empty()
        );
    }

    #[test]
    fn near_planets_excluding_fid_excludes_origin_planet() {
        let con = setup_search_db();

        let rows =
            near_planets_excluding_fid(&con, 1, 10.0, 10.0, 30.0, 10).expect("excluding fid");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].planet, "Tatooine");
    }

    #[test]
    fn update_unknown_planet_updates_requested_fields_and_planet_norm() {
        let con = setup_unknown_db();

        let updated = update_unknown_planet(
            &con,
            7,
            &UnknownPlanetUpdate {
                planet: Some("Ord Mantell".to_string()),
                region: Some(Some("Mid Rim".to_string())),
                sector: Some(None),
                system: Some(Some("Bright Jewel".to_string())),
                grid: Some(Some("M-12".to_string())),
                canon: Some(Some(1)),
                legends: Some(Some(0)),
                c_region: Some(Some("Mid Rim".to_string())),
                c_region_li: Some(Some("Mid Rim Territories".to_string())),
                reviewed: Some(1),
                notes: Some(Some("verified manually".to_string())),
            },
        )
        .expect("update succeeds");

        assert_eq!(updated.planet, "Ord Mantell");
        assert_eq!(updated.planet_norm, "ord mantell");
        assert_eq!(updated.region.as_deref(), Some("Mid Rim"));
        assert_eq!(updated.sector, None);
        assert_eq!(updated.system.as_deref(), Some("Bright Jewel"));
        assert_eq!(updated.grid.as_deref(), Some("M-12"));
        assert_eq!(updated.canon, Some(1));
        assert_eq!(updated.legends, Some(0));
        assert_eq!(updated.c_region.as_deref(), Some("Mid Rim"));
        assert_eq!(updated.c_region_li.as_deref(), Some("Mid Rim Territories"));
        assert_eq!(updated.reviewed, 1);
        assert_eq!(updated.notes.as_deref(), Some("verified manually"));
    }
}
