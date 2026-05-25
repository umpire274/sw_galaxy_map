use sqlx::{Executor, PgConnection};

/// Creates the standalone PostgreSQL v13 schema used by `sw_galaxy_map_sync`.
///
/// PostgreSQL does not use SQLite FTS5. Full-text support can be added later
/// with `tsvector` and GIN indexes.
pub async fn create_postgres_schema(conn: &mut PgConnection) -> anyhow::Result<()> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS planets (
            FID BIGINT PRIMARY KEY,
            Planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            Region TEXT,
            Sector TEXT,
            System TEXT,
            Grid TEXT,
            X DOUBLE PRECISION NOT NULL,
            Y DOUBLE PRECISION NOT NULL,
            arcgis_hash TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0,
            Canon INTEGER,
            Legends INTEGER,
            zm INTEGER,
            name0 TEXT,
            name1 TEXT,
            name2 TEXT,
            lat DOUBLE PRECISION,
            long DOUBLE PRECISION,
            ref TEXT,
            status TEXT,
            CRegion TEXT,
            CRegion_li TEXT,
            grid_unit TEXT NOT NULL DEFAULT 'pc',
            CHECK (Canon IS NULL OR Canon IN (0, 1)),
            CHECK (Legends IS NULL OR Legends IN (0, 1))
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS planets_unknown (
            id BIGSERIAL PRIMARY KEY,
            fid BIGINT,
            planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            region TEXT,
            sector TEXT,
            system TEXT,
            grid TEXT,
            x DOUBLE PRECISION,
            y DOUBLE PRECISION,
            arcgis_hash TEXT,
            deleted INTEGER NOT NULL DEFAULT 0,
            canon INTEGER,
            legends INTEGER,
            zm INTEGER,
            name0 TEXT,
            name1 TEXT,
            name2 TEXT,
            lat DOUBLE PRECISION,
            long DOUBLE PRECISION,
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
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS planet_aliases (
            id BIGSERIAL PRIMARY KEY,
            planet_fid BIGINT NOT NULL REFERENCES planets(FID) ON DELETE CASCADE,
            alias TEXT NOT NULL,
            alias_norm TEXT NOT NULL,
            source TEXT,
            UNIQUE (planet_fid, alias_norm)
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS planet_search (
            planet_fid BIGINT PRIMARY KEY REFERENCES planets(FID) ON DELETE CASCADE,
            planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            aliases TEXT,
            aliases_norm TEXT,
            search_text TEXT NOT NULL,
            search_norm TEXT NOT NULL
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS waypoints (
            id BIGSERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            name_norm TEXT NOT NULL,
            x DOUBLE PRECISION NOT NULL,
            y DOUBLE PRECISION NOT NULL,
            kind TEXT NOT NULL DEFAULT 'manual',
            fingerprint TEXT NOT NULL DEFAULT '',
            note TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS routes (
            id BIGSERIAL PRIMARY KEY,
            from_planet_fid BIGINT NOT NULL REFERENCES planets(FID),
            to_planet_fid BIGINT NOT NULL REFERENCES planets(FID),
            algo_version TEXT NOT NULL,
            options_json TEXT NOT NULL,
            length DOUBLE PRECISION,
            iterations INTEGER,
            status TEXT NOT NULL DEFAULT 'ok',
            error TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT,
            CHECK (status IN ('ok', 'failed'))
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS route_detours (
            route_id BIGINT NOT NULL REFERENCES routes(id) ON DELETE CASCADE,
            idx INTEGER NOT NULL,
            iteration INTEGER NOT NULL,
            segment_index INTEGER NOT NULL,
            obstacle_id BIGINT NOT NULL,
            obstacle_x DOUBLE PRECISION NOT NULL,
            obstacle_y DOUBLE PRECISION NOT NULL,
            obstacle_radius DOUBLE PRECISION NOT NULL,
            closest_t DOUBLE PRECISION NOT NULL,
            closest_qx DOUBLE PRECISION NOT NULL,
            closest_qy DOUBLE PRECISION NOT NULL,
            closest_dist DOUBLE PRECISION NOT NULL,
            offset_used DOUBLE PRECISION NOT NULL,
            wp_x DOUBLE PRECISION NOT NULL,
            wp_y DOUBLE PRECISION NOT NULL,
            waypoint_id BIGINT REFERENCES waypoints(id) ON DELETE SET NULL,
            score_base DOUBLE PRECISION NOT NULL,
            score_turn DOUBLE PRECISION NOT NULL,
            score_back DOUBLE PRECISION NOT NULL,
            score_proximity DOUBLE PRECISION NOT NULL,
            score_total DOUBLE PRECISION NOT NULL,
            tries_used INTEGER,
            tries_exhausted INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (route_id, idx)
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS route_waypoints (
            route_id BIGINT NOT NULL REFERENCES routes(id) ON DELETE CASCADE,
            seq INTEGER NOT NULL,
            x DOUBLE PRECISION NOT NULL,
            y DOUBLE PRECISION NOT NULL,
            waypoint_id BIGINT REFERENCES waypoints(id) ON DELETE SET NULL,
            PRIMARY KEY (route_id, seq)
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS waypoint_planets (
            waypoint_id BIGINT NOT NULL REFERENCES waypoints(id) ON DELETE CASCADE,
            planet_fid BIGINT NOT NULL REFERENCES planets(FID) ON DELETE CASCADE,
            role TEXT NOT NULL DEFAULT 'via',
            distance DOUBLE PRECISION,
            PRIMARY KEY (waypoint_id, planet_fid, role)
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
            CREATE TABLE IF NOT EXISTS planet_fid_remap (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                planet TEXT NOT NULL,
                local_fid INTEGER NOT NULL,
                remote_fid INTEGER NOT NULL,
                strategy TEXT NOT NULL,
                confidence REAL NOT NULL,
                approved INTEGER NOT NULL DEFAULT 0 CHECK(approved IN (0,1)),
                applied INTEGER NOT NULL DEFAULT 0 CHECK(applied IN (0,1)),
                created_at TEXT NOT NULL,
                UNIQUE(local_fid, remote_fid)
            );
        "#,
    )
    .await?;

    create_postgres_indexes(conn).await?;
    create_postgres_views(conn).await?;
    upsert_schema_version(conn).await?;

    Ok(())
}

async fn create_postgres_indexes(conn: &mut PgConnection) -> anyhow::Result<()> {
    let statements = [
        "CREATE INDEX IF NOT EXISTS idx_alias_norm ON planet_aliases (alias_norm)",
        "CREATE INDEX IF NOT EXISTS idx_alias_planetfid ON planet_aliases (planet_fid)",
        "CREATE INDEX IF NOT EXISTS idx_search_norm ON planet_search (search_norm)",
        "CREATE INDEX IF NOT EXISTS idx_search_planet_norm ON planet_search (planet_norm)",
        "CREATE INDEX IF NOT EXISTS idx_planets_grid_new ON planets (Grid)",
        "CREATE INDEX IF NOT EXISTS idx_planets_new ON planets (Planet, Region, Sector, System, X, Y)",
        "CREATE INDEX IF NOT EXISTS idx_planets_planet_new ON planets (Planet)",
        "CREATE INDEX IF NOT EXISTS idx_planets_planet_norm_new ON planets (planet_norm)",
        "CREATE INDEX IF NOT EXISTS idx_planets_region_new ON planets (Region)",
        "CREATE INDEX IF NOT EXISTS idx_planets_sector_new ON planets (Sector)",
        "CREATE INDEX IF NOT EXISTS idx_planets_system_new ON planets (System)",
        "CREATE INDEX IF NOT EXISTS idx_planets_xy_new ON planets (X, Y)",
        "CREATE UNIQUE INDEX IF NOT EXISTS ux_planets_unknown_fid ON planets_unknown (fid)",
        "CREATE INDEX IF NOT EXISTS idx_planets_unknown_planet ON planets_unknown (planet)",
        "CREATE INDEX IF NOT EXISTS idx_planets_unknown_planet_norm ON planets_unknown (planet_norm)",
        "CREATE INDEX IF NOT EXISTS idx_planets_unknown_promoted ON planets_unknown (promoted)",
        "CREATE INDEX IF NOT EXISTS idx_planets_unknown_reviewed ON planets_unknown (reviewed)",
        "CREATE INDEX IF NOT EXISTS idx_planets_unknown_sector ON planets_unknown (sector)",
        "CREATE INDEX IF NOT EXISTS idx_planets_unknown_system ON planets_unknown (system)",
        "CREATE INDEX IF NOT EXISTS idx_planets_unknown_xy ON planets_unknown (x, y)",
        "CREATE INDEX IF NOT EXISTS idx_routes_from_to ON routes (from_planet_fid, to_planet_fid, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_routes_status ON routes (status)",
        "CREATE UNIQUE INDEX IF NOT EXISTS ux_routes_from_to ON routes (from_planet_fid, to_planet_fid)",
        "CREATE INDEX IF NOT EXISTS idx_route_detours_route ON route_detours (route_id)",
        "CREATE INDEX IF NOT EXISTS idx_route_waypoints_route ON route_waypoints (route_id)",
        "CREATE INDEX IF NOT EXISTS idx_wp_planets_planet ON waypoint_planets (planet_fid)",
        "CREATE INDEX IF NOT EXISTS idx_wp_planets_role ON waypoint_planets (role)",
        "CREATE INDEX IF NOT EXISTS idx_wp_planets_waypoint ON waypoint_planets (waypoint_id)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_waypoints_fingerprint ON waypoints (fingerprint)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_waypoints_name_norm ON waypoints (name_norm)",
        "CREATE INDEX IF NOT EXISTS idx_waypoints_xy ON waypoints (x, y)",
        "CREATE INDEX IF NOT EXISTS idx_planet_fid_remap_planet ON planet_fid_remap(planet)",
        "CREATE INDEX IF NOT EXISTS idx_planet_fid_remap_approved ON planet_fid_remap(approved)",
        "CREATE INDEX IF NOT EXISTS idx_planet_fid_remap_applied ON planet_fid_remap(applied)",
    ];

    for statement in statements {
        conn.execute(statement).await?;
    }

    Ok(())
}

async fn create_postgres_views(conn: &mut PgConnection) -> anyhow::Result<()> {
    conn.execute(
        r#"
        CREATE OR REPLACE VIEW v_planets_clean AS
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
        ORDER BY p.Planet
        "#,
    )
    .await?;

    Ok(())
}

async fn upsert_schema_version(conn: &mut PgConnection) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO meta (key, value)
        VALUES ('schema_version', '14')
        ON CONFLICT (key) DO UPDATE SET
            value = EXCLUDED.value
        "#,
    )
    .execute(conn)
    .await?;

    Ok(())
}
