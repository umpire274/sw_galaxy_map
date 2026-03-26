use clap::{ArgAction, Args, Parser, Subcommand};

use sw_galaxy_map_core::domain::RouteListSort;

#[derive(Parser, Debug)]
#[command(
    name = "sw_galaxy_map",
    version,
    about = "CLI navicomputer for exploring the Star Wars galaxy (SQLite)",
    long_about = "\
Command-line navicomputer for exploring the Star Wars galaxy.

Default behavior:
  - Run without arguments to start the interactive CLI.
  - Use subcommands for one-shot CLI operations.

GUI startup is handled by the separate `sw_galaxy_map_gui` crate.
"
)]
pub struct Cli {
    /// Path to the SQLite database
    #[arg(long)]
    pub db: Option<String>,

    #[command(subcommand)]
    pub cmd: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search planets by text (uses FTS if available, otherwise LIKE)
    Search {
        /// Search query (name, description, features, ...)
        query: String,

        /// Max rows (default: 20)
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },

    /// Print all available information about a planet
    Info {
        /// Planet name (or alias)
        planet: String,
    },

    /// Find nearby planets within a radius (parsecs) using Euclidean distance on X/Y.
    ///
    /// Notes:
    /// - If you provide `--planet`, the planet coordinates are used as the center.
    /// - If you provide `--unknown`, the coordinates are read from `planets_unknown`.
    /// - Otherwise you must provide both `--x` and `--y`.
    /// - For negative coordinates, use the `=` form (e.g. `--y=-190`) to avoid CLI parsing ambiguity.
    Near {
        /// Reference planet name (positional)
        planet: Option<String>,

        /// Search radius (parsecs)
        #[arg(short = 'r', long = "range")]
        range: f64,

        /// Use unknown planets table
        #[arg(long)]
        unknown: bool,

        /// Reference FID (used with --unknown)
        #[arg(long)]
        fid: Option<i64>,

        /// X coordinate (alternative to planet)
        #[arg(long)]
        x: Option<f64>,

        /// Y coordinate (alternative to planet)
        #[arg(long)]
        y: Option<f64>,

        /// Limit number of results
        #[arg(long, default_value_t = 10)]
        limit: i64,
    },

    /// Database provisioning commands (C2: build local DB from remote data source)
    Db {
        #[command(subcommand)]
        cmd: DbCommands,
    },

    /// Manage waypoint catalog
    Waypoint {
        #[command(subcommand)]
        cmd: WaypointCmd,
    },

    /// Routing commands (router v1)
    Route {
        #[command(subcommand)]
        cmd: RouteCmd,
    },

    /// Work with unclassified planets stored in `planets_unknown`
    Unknown {
        #[command(subcommand)]
        cmd: UnknownCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum DbCommands {
    /// Initialize the local SQLite database by downloading data from the remote service
    Init {
        /// Output path for the generated SQLite database (defaults to OS app data dir)
        #[arg(long)]
        out: Option<String>,

        /// Overwrite existing database if present
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
    },

    /// Show local database status (path, meta, counts)
    Status,

    /// Update the local database with new data from the remote service
    Update {
        /// Permanently remove planets marked as deleted
        #[arg(long, action = ArgAction::SetTrue)]
        prune: bool,

        /// Perform a dry run without modifying the database
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,

        /// Show update statistics
        #[arg(long, action = ArgAction::SetTrue)]
        stats: bool,

        /// Limit for statistics output (default: 10)
        #[arg(long, default_value_t = 10)]
        stats_limit: usize,
    },

    /// Emit JSON listing the most recently skipped planets during db update
    SkippedPlanets,

    /// Migrate the local database to the latest schema version
    Migrate {
        /// Show what migrations would be applied without executing them
        #[arg(long, action = clap::ArgAction::SetTrue)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum UnknownCmd {
    /// List planets stored in `planets_unknown`
    List {
        /// Page number (starting from 1).
        #[arg(long, default_value_t = 1)]
        page: usize,

        /// Number of items per page.
        #[arg(long = "page-size", default_value_t = 25)]
        page_size: usize,
    },

    /// Search known planets near an unknown planet record
    Search {
        /// Internal unknown record ID
        id: i64,

        /// Radius in parsecs
        #[arg(long)]
        near: f64,

        /// Max rows (default: 20)
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },

    Near {
        /// Reference planet name
        planet: String,

        /// Search radius (parsecs)
        #[arg(short = 'r', long = "range")]
        range: f64,

        /// Limit number of results
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },

    /// Edit an unknown planet record in `planets_unknown`
    Edit {
        /// Internal unknown record ID
        id: i64,

        /// Planet name
        #[arg(long)]
        planet: Option<String>,

        /// Region
        #[arg(long)]
        region: Option<String>,

        /// Sector
        #[arg(long)]
        sector: Option<String>,

        /// System
        #[arg(long)]
        system: Option<String>,

        /// Grid
        #[arg(long)]
        grid: Option<String>,

        /// Canon flag (true/false)
        #[arg(long)]
        canon: Option<bool>,

        /// Legends flag (true/false)
        #[arg(long)]
        legend: Option<bool>,

        /// Canonical region
        #[arg(long = "cregion")]
        c_region: Option<String>,

        /// Canonical region (long label)
        #[arg(long = "cregion-li")]
        c_region_li: Option<String>,

        /// Reviewed flag (true/false)
        #[arg(long)]
        reviewed: Option<bool>,

        /// Free-form notes
        #[arg(long)]
        notes: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum WaypointCmd {
    /// Add a new waypoint
    Add {
        /// Waypoint name (unique, human friendly)
        name: String,

        /// X coordinate (parsec)
        #[arg(allow_hyphen_values = true)]
        x: f64,

        /// Y coordinate (parsec)
        #[arg(allow_hyphen_values = true)]
        y: f64,

        /// Waypoint kind (manual, junction, nav_buoy, computed, ...)
        #[arg(long, default_value = "manual")]
        kind: String,

        /// Optional note
        #[arg(long)]
        note: Option<String>,
    },

    /// List waypoints
    List {
        /// Max rows (default: 50)
        #[arg(long, default_value_t = 50)]
        limit: usize,

        /// Offset (default: 0)
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },

    /// Show waypoint details by name (normalized) or by id
    Show {
        /// Waypoint name (e.g. "Corellian Junction") or numeric id (e.g. "12")
        key: String,
    },

    /// Delete waypoint by id
    Delete {
        /// Waypoint id
        id: i64,
    },

    /// Link a waypoint to a planet (planet name or alias)
    Link {
        /// Waypoint ID
        waypoint_id: i64,

        /// Planet name or alias
        planet: String,

        /// Role of the planet for this waypoint (default: "anchor")
        #[arg(long, default_value = "anchor")]
        role: String,

        /// Optional distance (parsec). If omitted, it can be computed later.
        #[arg(long)]
        distance: Option<f64>,
    },

    /// List planet links for a waypoint
    Links {
        /// Waypoint ID
        waypoint_id: i64,
    },

    /// List waypoints linked to a planet (planet name or alias)
    ForPlanet {
        /// Planet name or alias
        planet: String,

        /// Optional role filter
        #[arg(long)]
        role: Option<String>,

        /// Max rows (default: 50)
        #[arg(long, default_value_t = 50)]
        limit: usize,

        /// Offset (default: 0)
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },

    /// Unlink a waypoint from a planet
    Unlink {
        /// Waypoint ID
        waypoint_id: i64,

        /// Planet name or alias
        planet: String,
    },

    /// Remove orphan computed waypoints (not referenced by any route)
    Prune {
        /// Do not delete anything, just show what would be deleted
        #[arg(long)]
        dry_run: bool,

        /// Also prune computed waypoints even if they have planet links (waypoint_planets).
        /// Links will be removed as part of the prune.
        #[arg(long)]
        include_linked: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum RouteCmd {
    /// Compute and persist a route between two or more planets (name or alias)
    Compute(RouteComputeArgs),

    /// Show a persisted route by id
    Show {
        /// Route id
        route_id: i64,
    },

    /// Explain a persisted route detours (why/what/how) by id
    Explain(RouteExplainArgs),

    /// Show the current persisted route for a FROM→TO pair (unique in schema v8)
    Last {
        /// Start planet name (or alias)
        from: String,

        /// Destination planet name (or alias)
        to: String,
    },

    /// Clear all persisted routes (routes, waypoints, detours)
    Clear {
        /// Skip interactive confirmation prompt (destructive)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        yes: bool,
    },

    /// Prune orphan rows in route_waypoints / route_detours not linked to any route
    Prune,

    // ...
    List {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        json: bool,

        #[arg(long, requires = "json")]
        file: Option<std::path::PathBuf>,

        #[arg(long, default_value_t = 50)]
        limit: usize,

        /// Filter by status (e.g. ok, failed)
        #[arg(long)]
        status: Option<String>,

        /// Filter by FROM planet fid
        #[arg(long)]
        from: Option<i64>,

        /// Filter by TO planet fid
        #[arg(long)]
        to: Option<i64>,

        /// Filter by exact number of waypoints
        #[arg(long)]
        wp: Option<usize>,

        /// Sort field (updated|id|length). Default: updated
        #[arg(long, value_enum, default_value_t = RouteListSort::Updated)]
        sort: RouteListSort,
    },
}

#[derive(Args, Debug)]
pub struct RouteComputeArgs {
    /// Planet names (or aliases), in travel order
    #[arg(required = true, num_args = 2.., value_name = "PLANET")]
    pub planets: Vec<String>,

    /// Safety radius in parsecs used to model a planet's hyperspace no-fly zone.
    ///
    /// During hyperspace navigation, planets are treated as circular obstacles with this radius,
    /// representing gravitational mass shadows, hyperspace shear, interdiction effects,
    /// and standard navigational safety margins used by astrogators.
    ///
    /// This value does NOT represent the physical radius of the planet.
    /// Larger values produce safer but longer routes with more detours,
    /// while smaller values favor more direct (and riskier) trajectories.
    ///
    /// Default: 2.0 parsecs
    #[arg(long, default_value_t = 2.0)]
    pub safety: f64,

    /// Extra clearance beyond obstacle radius when generating detours
    #[arg(long, default_value_t = 0.2)]
    pub clearance: f64,

    #[arg(long, default_value_t = 32)]
    pub max_iters: usize,

    #[arg(long, default_value_t = 6)]
    pub max_offset_tries: usize,

    #[arg(long, default_value_t = 1.4)]
    pub offset_growth: f64,

    /// Penalize sharp turns
    #[arg(long, default_value_t = 0.8)]
    pub turn_weight: f64,

    /// Penalize moving backward relative to A->B direction
    #[arg(long, default_value_t = 3.0)]
    pub back_weight: f64,

    /// Penalize getting close to other obstacles (soft constraint)
    #[arg(long, default_value_t = 1.5)]
    pub proximity_weight: f64,

    /// Extra band beyond obstacle radius for proximity penalty
    #[arg(long, default_value_t = 0.5)]
    pub proximity_margin: f64,

    /// Bounding box margin (parsec) around the segment A->B to fetch candidate obstacles
    #[arg(long, default_value_t = 80.0)]
    pub bbox_margin: f64,

    /// Max obstacles to consider (debug safety cap)
    #[arg(long, default_value_t = 8000)]
    pub max_obstacles: usize,
}

#[derive(Args, Debug)]
pub struct RouteExplainArgs {
    /// Route id
    pub route_id: i64,

    /// Export explanation as JSON (stdout)
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub json: bool,

    /// Write JSON to file (absolute or relative path). Requires --json.
    #[arg(long, requires = "json")]
    pub file: Option<std::path::PathBuf>,

    /// Hyperdrive class (e.g. 0.5, 1.0, 2.0)
    #[arg(long = "class", default_value_t = 1.0)]
    pub hyperdrive_class: f64,

    /// Region blend strategy: avg | conservative | <from_weight>
    #[arg(long = "region-blend", default_value = "avg")]
    pub region_blend: String,

    /// Include a sublight ETA using the given speed (km/s).
    ///
    /// Example: `--sublight-kmps 2000` (civilian-ish baseline)
    #[arg(long = "sublight-kmps")]
    pub sublight_kmps: Option<f64>,
}
