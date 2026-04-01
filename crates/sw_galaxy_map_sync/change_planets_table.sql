create table if not exists main.planets_new
(
    FID         INTEGER
        primary key,
    Planet      TEXT              not null,
    planet_norm TEXT              not null,
    Region      TEXT,
    Sector      TEXT,
    System      TEXT,
    Grid        TEXT,
    X           REAL              not null,
    Y           REAL              not null,
    arcgis_hash TEXT              not null,
    Canon       INTEGER,
    Legends     INTEGER,
    zm          INTEGER,
    name0       TEXT,
    name1       TEXT,
    name2       TEXT,
    lat         REAL,
    long        REAL,
    ref         TEXT,
    status      TEXT,
    CRegion     TEXT,
    CRegion_li  TEXT,
    check (Canon IS NULL OR Canon IN (0, 1)),
    check (Legends IS NULL OR Legends IN (0, 1))
);

create index if not exists idx_planets_new
    on planets_new (Planet, Region, Sector, System, X, Y);

create index if not exists idx_planets_grid_new
    on planets_new (Grid);

create index if not exists idx_planets_planet_new
    on planets_new (Planet);

create index if not exists idx_planets_planet_norm_new
    on planets_new (planet_norm);

create index if not exists idx_planets_region_new
    on planets_new (Region);

create index if not exists idx_planets_sector_new
    on planets_new (Sector);

create index if not exists idx_planets_system_new
    on planets_new (System);

create index if not exists idx_planets_xy_new
    on planets_new (X, Y);

INSERT INTO planets_new (
    FID, Planet, planet_norm, Region, Sector, System, Grid, X, Y,
    arcgis_hash, Canon, Legends, zm, name0, name1, name2, lat, long, ref,
    status, CRegion, CRegion_li
)
SELECT
    FID, Planet, planet_norm, Region, Sector, System, Grid, X, Y,
    arcgis_hash, Canon, Legends, zm, name0, name1, name2, lat, long, ref,
    status, CRegion, CRegion_li
FROM planets;

DROP TABLE planets;
DROP VIEW v_planets_clean;

ALTER TABLE planets_new RENAME TO planets;

CREATE VIEW v_planets_clean as
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
WHERE p.status != "deleted"
ORDER BY p.Planet COLLATE NOCASE;

