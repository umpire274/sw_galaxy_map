use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "sw_galaxy_map",
    version,
    about = "CLI to query the Star Wars galaxy map (SQLite)",
    long_about = "\
Command-line and graphical navicomputer for exploring the Star Wars galaxy.

Run without arguments to start the graphical navicomputer interface.
"
)]
pub struct Cli {
    /// Path to the SQLite database
    #[arg(long)]
    pub db: Option<String>,

    #[command(subcommand)]
    pub cmd: Commands,
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
    /// - Otherwise you must provide both `--x` and `--y`.
    /// - For negative coordinates, use the `=` form (e.g. `--y=-190`) to avoid CLI parsing ambiguity.
    Near {
        /// Radius (parsecs)
        #[arg(long)]
        r: f64,

        /// Center the search around a planet (by name)
        #[arg(long)]
        planet: Option<String>,

        /// X coordinate (parsecs), if --planet is not used.
        ///
        /// Tip: for negative values use `--x=-190` (with '=').
        #[arg(long, verbatim_doc_comment)]
        x: Option<f64>,

        /// Y coordinate (parsecs), if --planet is not used.
        ///
        /// Tip: for negative values use `--y=-190` (with '=').
        #[arg(long, verbatim_doc_comment)]
        y: Option<f64>,

        /// Max rows (default: 20)
        #[arg(long, default_value_t = 20)]
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

    /// Migrate the local database to the latest schema version
    Migrate {
        /// Show what migrations would be applied without executing them
        #[arg(long, action = clap::ArgAction::SetTrue)]
        dry_run: bool,
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
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum RouteListSort {
    Updated,
    Id,
    Length,
}

#[derive(Subcommand, Debug)]
pub enum RouteCmd {
    /// Compute and persist a route between two planets (name or alias)
    Compute(RouteComputeArgs),

    /// Show a persisted route by id
    Show {
        /// Route id
        route_id: i64,
    },

    /// Explain a persisted route detours (why/what/how) by id
    Explain {
        /// Route id
        route_id: i64,

        /// Export explanation as JSON (stdout)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        json: bool,

        /// Write JSON to file (absolute or relative path). Requires --json.
        #[arg(long, requires = "json")]
        file: Option<std::path::PathBuf>,
    },

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
    /// Start planet name (or alias)
    pub from: String,

    /// Destination planet name (or alias)
    pub to: String,

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
