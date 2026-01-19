# sw_galaxy_map

**sw_galaxy_map** is a command-line application written in Rust that allows querying
and exploring the Star Wars galaxy using a local SQLite database.

The application provides tools to:

- search for planets by name or alias,
- display all available information about a specific planet,
- find nearby planets within a given radius using Euclidean distance
  on X/Y coordinates expressed in parsecs.

The project is designed as an offline, fast, and script-friendly CLI tool,
intended primarily for educational and non-commercial use.

---

## Acknowledgements

The planetary data used by this project were obtained from the **Star Wars Galaxy Map**
available at:

[Star Wars Galaxy Map](http://www.swgalaxymap.com/): Explore the Galaxy Far, Far Away

The Star Wars Galaxy Map project is created and maintained by **Henry Bernberg**.
All credit for the original dataset, research, and compilation goes to him.

If you find this data valuable, please consider supporting the original author via one
of the official donation channels:

- [Ko-fi](https://ko-fi.com/J3J0197XZ)
- [PayPal](https://www.paypal.com/donate?token=rk-LV-u5miGM2sumnvRL5ZiAFjnwIhhLnsSe-mqEnFgDAmeIhBkG6CQamxUxUoR18iwI0mA8h5ruuIk_)

This project uses the data for **educational and non-commercial purposes** only and
is not affiliated with, endorsed by, or associated with the Star Wars Galaxy Map
website, Lucasfilm, or The Walt Disney Company.

---

## 🧭 Route computation & detour waypoints

The routing engine computes hyperspace routes between two planets using a 2D galactic map (X/Y coordinates in parsecs).
The ideal route is a straight line between the origin and the destination; however, planets generate **hyperspace no-fly
zones** that cannot be crossed.

When a route intersects one or more of these zones, the engine dynamically inserts **detour waypoints** to safely bypass
the obstacle.

### 🌌 Planet obstacles model

- Each planet is treated as a **circular obstacle** in the galactic plane.
- The obstacle radius is defined by the `--safety` parameter and represents:
    - gravitational mass shadows
    - hyperspace shear
    - interdiction fields
    - standard astrogation safety margins

- This radius is **not** the physical size of the planet.

### 🔍 Collision detection

For each segment of the current route polyline:

1. The engine computes the **closest point on the segment** to every nearby planet.
2. If this distance is less than the planet’s safety radius, the segment is considered in **hard collision**.
3. The first collision along the route (lowest parametric `t`) is resolved before any others.

### 🧩 Detour candidate generation

When a collision is detected on a segment **A** → **B**, the engine generates a set of candidate detour waypoints around
the collision point **Q**.
Candidates are placed using different directions to ensure robustness in dense regions:

- **Radial** (**away from planet center**)
  Strongly preferred: pushes the route directly outside the no-fly zone.

- **Lateral** (**left/right of the segment**)
  Classic bypass around the obstacle.

- **Forward** / **backward** (**along the route direction**)
  Useful in clustered systems.

- Diagonal directions
  Improve chances of escaping complex obstacle layouts.

The distance of each candidate from the collision point is:

```ini
offset = obstacle_radius + clearance
```

If no valid candidate is found, the offset is progressively increased using:

```ini
offset *= offset_growth
```

This process is repeated up to `max_offset_tries`.

### ⚖️ Candidate evaluation & scoring

Each candidate waypoint **W** is evaluated by splitting the segment into:

- **A** → **W**
- **W** → **B**

Both segments must be collision-free.

Valid candidates are scored using a weighted cost function:

1. **Base length**

Total path length increase:

```ini
base = |A→W| + |W→B|
```

2. **Turn penalty**

Penalizes sharp angles (zig-zag routes):

```ini
turn = turn_weight × (1 − cosθ)
```

3. **Backtracking penalty**

Penalizes detours that move backward relative to the overall direction:

```ini
back = back_weight × max(0, −dot(dir(A→B), dir(A→W)))
```

4. **Proximity penalty**

Soft penalty for passing close to other planets (even without collision):

- Applies within a configurable warning band:
  ```ini
    warning_radius = obstacle_radius + proximity_margin
  ```

- Grows quadratically as the route approaches the obstacle.

**Total score**

```ini
total = base + turn + back + proximity
```

The candidate with the **lowest total score** is selected.

### 🧠 Iterative routing

- The chosen waypoint is inserted into the route.
- The process restarts from the beginning of the polyline.
- The algorithm stops when:
    - no collisions remain, or
    - max_iters is reached.

Each detour decision (collision, chosen waypoint, score breakdown) is recorded and can be:

- printed in debug output
- visualized
- persisted to the database

### ✨ Result

The final route consists of:

- a polyline of waypoints (start, detours, destination)
- total route length
- a chronological list of detour decisions explaining why each waypoint exists

This approach produces routes that are:

- safe
- explainable
- stable
- suitable for both CLI usage and future visualization layers

---

## 🗄️ Persistence model (SQLite DB design)

The project persists computed routes and generated detour waypoints in the local SQLite database to support:

- caching (avoid recomputing the same route repeatedly)
- inspection/debugging (`route show`, `route last`)
- future visualization (map rendering, route replay, analytics)
- building a growing catalog of navigation waypoints

Routes and waypoints are persisted using the following tables.

### 📍 waypoints

Global waypoint catalog. It contains both:

- **manual waypoints** (user-defined: junctions, buoys, etc.)
- **computed waypoints** (generated by the router when bypassing obstacles)

Each waypoint is a point in the galactic plane:

| Column        | Type       | Notes                                           |
|---------------|------------|-------------------------------------------------|
| `id`          | INTEGER PK | Waypoint identifier                             |
| `name`        | TEXT       | Human-friendly name                             |
| `name_norm`   | TEXT       | Normalized name (unique lookup key)             |
| `x`, `y`      | REAL       | Coordinates in parsecs                          |
| `kind`        | TEXT       | e.g. `manual`, `junction`, `computed`           |
| `fingerprint` | TEXT NULL  | **Only for computed** waypoints; used for dedup |
| `note`        | TEXT NULL  | Optional free text                              |
| `created_at`  | TEXT       | UTC timestamp                                   |
| `updated_at`  | TEXT NULL  | UTC timestamp                                   |

**Computed waypoint deduplication** (`fingerprint`)
Computed waypoints may be generated multiple times across different routes or parameter sets.
To avoid inserting duplicates, computed waypoints use a stable **fingerprint** (hash/string) that represents the
generated
waypoint identity (e.g. geometry + obstacle context + offset model).

When persisting computed waypoints the engine performs an **upsert** keyed by `fingerprint`:

- if a waypoint with the same fingerprint exists → reuse it
- otherwise → insert a new computed waypoint

This yields a growing, reusable navigation catalog over time.

### 🪐 waypoint_planets

Associates a waypoint with one or more nearby/related planets.

This is useful for:

- “anchor” planets for a waypoint (e.g. “near Corellia”)
- computed waypoints generated while bypassing specific obstacles
- future UI/visualization (grouping waypoints around systems)

| Column        | Type                        | Notes                             |
|---------------|-----------------------------|-----------------------------------|
| `id`          | INTEGER PK                  | Link identifier                   |
| `waypoint_id` | INTEGER FK → `waypoints.id` | Waypoint                          |
| `planet_fid`  | INTEGER FK → `planets.FID`  | Planet                            |
| `role`        | TEXT                        | e.g. `anchor`, `near`, `obstacle` |
| `distance`    | REAL NULL                   | Optional distance (parsecs)       |
| `created_at`  | TEXT                        | UTC timestamp                     |

A waypoint can be linked to multiple planets, and a planet can have multiple related waypoints.

### 🧭 routes

Routes are persisted as “cache entries” between a pair of planets.

**Important design rule**: routes are **unique per origin/destination** pair.

This ensures that:

- running the same route again with different parameters updates the record
- the database does not accumulate duplicates for the same FROM → TO
- `route last <from> <to>` is well-defined

| Column            | Type                       | Notes                                                          |
|-------------------|----------------------------|----------------------------------------------------------------|
| `id`              | INTEGER PK                 | Route identifier                                               |
| `from_planet_fid` | INTEGER FK → `planets.FID` | Origin                                                         |
| `to_planet_fid`   | INTEGER FK → `planets.FID` | Destination                                                    |
| `algo_version`    | TEXT                       | Router version string (e.g. `router_v1`)                       |
| `options_json`    | TEXT                       | Serialized options used (`safety`, `clearance`, weights, etc.) |
| `length`          | REAL NULL                  | Total route length (parsecs)                                   |
| `iterations`      | INTEGER NULL               | Router iterations                                              |
| `status`          | TEXT                       | `ok` / `error`                                                 |
| `error`           | TEXT NULL                  | Error details if status=`error`                                |
| `created_at`      | TEXT                       | UTC timestamp                                                  |
| `updated_at`      | TEXT NULL                  | UTC timestamp                                                  |

Uniqueness is enforced by a unique index:

- `(from_planet_fid, to_planet_fid)` is unique

**Upsert semantics**

Recomputing a route performs an **UPSERT**:

- update route metadata (`options_json`, `length`, etc.)
- update `updated_at`
- replace route polyline and detour details (see below)

### 🧷 route_waypoints

Stores the final polyline (ordered list of points) for a route.

Each row represents one waypoint in the route, identified by its sequence index (`seq`).

Waypoints may refer to entries in the global waypoint catalog (`waypoints.id`) or may be “raw” coordinates.

| Column        | Type                             | Notes                          |
|---------------|----------------------------------|--------------------------------|
| `id`          | INTEGER PK                       | Row identifier                 |
| `route_id`    | INTEGER FK → `routes.id`         | Route                          |
| `seq`         | INTEGER                          | Order in the polyline (0..N-1) |
| `x`, `y`      | REAL                             | Waypoint coordinates           |
| `waypoint_id` | INTEGER NULL FK → `waypoints.id` | Optional link to catalog       |
| `kind`        | TEXT                             | e.g. `start`, `detour`, `end`  |
| `created_at`  | TEXT                             | UTC timestamp                  |

**Replace strategy**
On route recomputation, the engine removes and reinserts the polyline:

- `DELETE FROM route_waypoints WHERE route_id = ?`
- insert the new ordered polyline

This keeps the stored route consistent with the latest `options_json`.

### 🧠 route_detours

Stores detailed decisions made by the router while building the route.

Detours are persisted to enable:

- debugging and replay
- future visualization (e.g. show which planet caused each detour)
- metrics and tuning (scores, offsets)

| Column                                                                     | Type                     | Notes                                |
|----------------------------------------------------------------------------|--------------------------|--------------------------------------|
| `id`                                                                       | INTEGER PK               | Row identifier                       |
| `route_id`                                                                 | INTEGER FK → `routes.id` | Route                                |
| `idx`                                                                      | INTEGER                  | Detour index (chronological order)   |
| `iteration`                                                                | INTEGER                  | Router iteration                     |
| `segment_index`                                                            | INTEGER                  | Which segment was split              |
| `obstacle_id`                                                              | INTEGER                  | Planet fid treated as obstacle       |
| `obstacle_x`, `obstacle_y`                                                 | REAL                     | Obstacle center                      |
| `obstacle_radius`                                                          | REAL                     | Safety radius used                   |
| `closest_t`                                                                | REAL                     | Parametric t of closest point        |
| `closest_qx`, `closest_qy`                                                 | REAL                     | Closest point on segment             |
| `closest_dist`                                                             | REAL                     | Distance at collision time           |
| `offset_used`                                                              | REAL                     | Offset used for candidate generation |
| `wp_x`, `wp_y`                                                             | REAL                     | Chosen detour waypoint               |
| `score_base`, `score_turn`, `score_back`, `score_proximity`, `score_total` | REAL                     | Score breakdown                      |
| `created_at`                                                               | TEXT                     | UTC timestamp                        |

**Replace strategy**

Same as route polyline:

- `DELETE FROM route_detours WHERE route_id = ?`
- insert the new detour decision list

### ✅ Persistence workflow (high-level)

When running `route compute <from> <to>`:
1. Resolve `from/to` to planet fids. 
2. Compute the route in memory. 
3. Upsert the route record in `routes` (unique per pair).
4. Replace the associated polyline in `route_waypoints`. 
5. Replace the associated decision log in `route_detours`. 
6. For each detour waypoint:
   - upsert into `waypoints` if computed (via `fingerprint`)
   - optionally link waypoint ↔ planets in `waypoint_planets`

This ensures that the database always holds the most recent “best known” route for each pair.

