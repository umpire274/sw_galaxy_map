use rusqlite::Connection;

/// Creates the standalone SQLite v13 schema used by `sw_galaxy_map_sync`.
pub fn create_sqlite_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS planets (
            FID INTEGER PRIMARY KEY,
            Planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            Region TEXT,
            Sector TEXT,
            System TEXT,
            Grid TEXT,
            X REAL NOT NULL,
            Y REAL NOT NULL,
            arcgis_hash TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0,
            Canon INTEGER,
            Legends INTEGER,
            zm INTEGER,
            name0 TEXT,
            name1 TEXT,
            name2 TEXT,
            lat REAL,
            long REAL,
            ref TEXT,
            status TEXT,
            CRegion TEXT,
            CRegion_li TEXT,
            grid_unit TEXT NOT NULL DEFAULT 'pc',
            CHECK (Canon IS NULL OR Canon IN (0, 1)),
            CHECK (Legends IS NULL OR Legends IN (0, 1))
        );

        CREATE TABLE IF NOT EXISTS planets_unknown (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fid INTEGER,
            planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            region TEXT,
            sector TEXT,
            system TEXT,
            grid TEXT,
            x REAL,
            y REAL,
            arcgis_hash TEXT,
            deleted INTEGER NOT NULL DEFAULT 0,
            canon INTEGER,
            legends INTEGER,
            zm INTEGER,
            name0 TEXT,
            name1 TEXT,
            name2 TEXT,
            lat REAL,
            long REAL,
            ref TEXT,
            status TEXT,
            cregion TEXT,
            cregion_li TEXT,
            reason TEXT,
            reviewed INTEGER NOT NULL DEFAULT 0,
            promoted INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            grid_unit TEXT NOT NULL DEFAULT 'pc',
            CHECK (canon IS NULL OR canon IN (0, 1)),
            CHECK (deleted IN (0, 1)),
            CHECK (legends IS NULL OR legends IN (0, 1)),
            CHECK (promoted IN (0, 1)),
            CHECK (reviewed IN (0, 1))
        );

        CREATE TABLE IF NOT EXISTS planet_aliases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            planet_fid INTEGER NOT NULL REFERENCES planets ON DELETE CASCADE,
            alias TEXT NOT NULL,
            alias_norm TEXT NOT NULL,
            source TEXT,
            UNIQUE (planet_fid, alias_norm)
        );

        CREATE TABLE IF NOT EXISTS planet_search (
            planet_fid INTEGER PRIMARY KEY REFERENCES planets ON DELETE CASCADE,
            planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            aliases TEXT,
            aliases_norm TEXT,
            search_text TEXT NOT NULL,
            search_norm TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS routes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_planet_fid INTEGER NOT NULL REFERENCES planets,
            to_planet_fid INTEGER NOT NULL REFERENCES planets,
            algo_version TEXT NOT NULL,
            options_json TEXT NOT NULL,
            length REAL,
            iterations INTEGER,
            status TEXT NOT NULL DEFAULT 'ok',
            error TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            updated_at TEXT,
            CHECK (status IN ('ok', 'failed'))
        );

        CREATE TABLE IF NOT EXISTS waypoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            name_norm TEXT NOT NULL,
            x REAL NOT NULL,
            y REAL NOT NULL,
            kind TEXT NOT NULL DEFAULT 'manual',
            fingerprint TEXT NOT NULL DEFAULT '',
            note TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS route_detours (
            route_id INTEGER NOT NULL REFERENCES routes ON DELETE CASCADE,
            idx INTEGER NOT NULL,
            iteration INTEGER NOT NULL,
            segment_index INTEGER NOT NULL,
            obstacle_id INTEGER NOT NULL,
            obstacle_x REAL NOT NULL,
            obstacle_y REAL NOT NULL,
            obstacle_radius REAL NOT NULL,
            closest_t REAL NOT NULL,
            closest_qx REAL NOT NULL,
            closest_qy REAL NOT NULL,
            closest_dist REAL NOT NULL,
            offset_used REAL NOT NULL,
            wp_x REAL NOT NULL,
            wp_y REAL NOT NULL,
            waypoint_id INTEGER REFERENCES waypoints ON DELETE SET NULL,
            score_base REAL NOT NULL,
            score_turn REAL NOT NULL,
            score_back REAL NOT NULL,
            score_proximity REAL NOT NULL,
            score_total REAL NOT NULL,
            tries_used INTEGER,
            tries_exhausted INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (route_id, idx)
        );

        CREATE TABLE IF NOT EXISTS route_waypoints (
            route_id INTEGER NOT NULL REFERENCES routes ON DELETE CASCADE,
            seq INTEGER NOT NULL,
            x REAL NOT NULL,
            y REAL NOT NULL,
            waypoint_id INTEGER REFERENCES waypoints ON DELETE SET NULL,
            PRIMARY KEY (route_id, seq)
        );

        CREATE TABLE IF NOT EXISTS waypoint_planets (
            waypoint_id INTEGER NOT NULL REFERENCES waypoints ON DELETE CASCADE,
            planet_fid INTEGER NOT NULL REFERENCES planets ON DELETE CASCADE,
            role TEXT NOT NULL DEFAULT 'via',
            distance REAL,
            PRIMARY KEY (waypoint_id, planet_fid, role)
        );

        CREATE INDEX IF NOT EXISTS idx_alias_norm ON planet_aliases (alias_norm);
        CREATE INDEX IF NOT EXISTS idx_alias_planetfid ON planet_aliases (planet_fid);

        CREATE INDEX IF NOT EXISTS idx_search_norm ON planet_search (search_norm);
        CREATE INDEX IF NOT EXISTS idx_search_planet_norm ON planet_search (planet_norm);

        CREATE INDEX IF NOT EXISTS idx_planets_grid_new ON planets (Grid);
        CREATE INDEX IF NOT EXISTS idx_planets_new ON planets (Planet, Region, Sector, System, X, Y);
        CREATE INDEX IF NOT EXISTS idx_planets_planet_new ON planets (Planet);
        CREATE INDEX IF NOT EXISTS idx_planets_planet_norm_new ON planets (planet_norm);
        CREATE INDEX IF NOT EXISTS idx_planets_region_new ON planets (Region);
        CREATE INDEX IF NOT EXISTS idx_planets_sector_new ON planets (Sector);
        CREATE INDEX IF NOT EXISTS idx_planets_system_new ON planets (System);
        CREATE INDEX IF NOT EXISTS idx_planets_xy_new ON planets (X, Y);

        CREATE INDEX IF NOT EXISTS idx_planets_unknown_fid ON planets_unknown (fid);
        CREATE INDEX IF NOT EXISTS idx_planets_unknown_planet ON planets_unknown (planet);
        CREATE INDEX IF NOT EXISTS idx_planets_unknown_planet_norm ON planets_unknown (planet_norm);
        CREATE INDEX IF NOT EXISTS idx_planets_unknown_promoted ON planets_unknown (promoted);
        CREATE INDEX IF NOT EXISTS idx_planets_unknown_reviewed ON planets_unknown (reviewed);
        CREATE INDEX IF NOT EXISTS idx_planets_unknown_sector ON planets_unknown (sector);
        CREATE INDEX IF NOT EXISTS idx_planets_unknown_system ON planets_unknown (system);
        CREATE INDEX IF NOT EXISTS idx_planets_unknown_xy ON planets_unknown (x, y);

        CREATE INDEX IF NOT EXISTS idx_routes_from_to ON routes (from_planet_fid, to_planet_fid, created_at);
        CREATE INDEX IF NOT EXISTS idx_routes_status ON routes (status);
        CREATE UNIQUE INDEX IF NOT EXISTS ux_routes_from_to ON routes (from_planet_fid, to_planet_fid);

        CREATE INDEX IF NOT EXISTS idx_route_detours_route ON route_detours (route_id);
        CREATE INDEX IF NOT EXISTS idx_route_waypoints_route ON route_waypoints (route_id);

        CREATE INDEX IF NOT EXISTS idx_wp_planets_planet ON waypoint_planets (planet_fid);
        CREATE INDEX IF NOT EXISTS idx_wp_planets_role ON waypoint_planets (role);
        CREATE INDEX IF NOT EXISTS idx_wp_planets_waypoint ON waypoint_planets (waypoint_id);

        CREATE UNIQUE INDEX IF NOT EXISTS idx_waypoints_fingerprint ON waypoints (fingerprint);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_waypoints_name_norm ON waypoints (name_norm);
        CREATE INDEX IF NOT EXISTS idx_waypoints_xy ON waypoints (x, y);

        CREATE TRIGGER IF NOT EXISTS trg_waypoints_updated_at
        AFTER UPDATE ON waypoints
        FOR EACH ROW
        BEGIN
            UPDATE waypoints SET updated_at = datetime('now') WHERE id = OLD.id;
        END;

        CREATE VIEW IF NOT EXISTS v_planets_clean AS
        SELECT
            p.FID,
            p.Planet,
            p.Region,
            p.Sector,
            p.System,
            p.Grid,
            p.X AS x_parsec,
            p.Y AS y_parsec,
            p.Canon,
            p.Legends,
            p.status,
            p.ref
        FROM planets p
        WHERE p.status NOT IN ('deleted', 'skipped', 'invalid')
        ORDER BY p.Planet COLLATE NOCASE;
        "#,
    )?;

    conn.execute(
        r#"
        INSERT INTO meta (key, value)
        VALUES ('schema_version', '13')
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        [],
    )?;

    create_sqlite_fts_if_available(conn)?;

    Ok(())
}

fn create_sqlite_fts_if_available(conn: &Connection) -> anyhow::Result<()> {
    if sqlite_has_fts5(conn)? {
        conn.execute_batch(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS planets_fts USING fts5 (
                planet_fid UNINDEXED,
                search_norm,
                tokenize = 'unicode61'
            );
            "#,
        )?;
    }

    Ok(())
}

fn sqlite_has_fts5(conn: &Connection) -> anyhow::Result<bool> {
    let enabled: i64 = conn.query_row(
        "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
        [],
        |row| row.get(0),
    )?;

    Ok(enabled == 1)
}
