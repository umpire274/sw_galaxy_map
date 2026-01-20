use anyhow::{Result, bail};

use crate::routing::collision::*;
use crate::routing::geometry::*;
use crate::routing::route_debug::debug_failed_detour;

#[derive(Debug, Clone)]
pub struct DetourDecision {
    /// Iteration number (0..)
    pub iteration: usize,

    /// Index of the segment in the polyline that was split (A = waypoints[idx], B = waypoints[idx+1])
    pub segment_index: usize,

    /// Obstacle being bypassed
    pub obstacle_id: i64,
    pub obstacle_name: String,
    pub obstacle_center: Point,
    pub obstacle_radius: f64,

    /// Closest point information at collision time (useful for debugging/visualization)
    pub closest_t: f64,
    pub closest_q: Point,
    pub closest_dist: f64,

    /// Offset used for the chosen detour
    pub offset_used: f64,

    /// Chosen waypoint inserted
    pub waypoint: Point,

    /// Score breakdown for the chosen waypoint
    pub score: CandidateScore,

    /// Number of offset-try iterations consumed before a valid detour was found.
    /// This is provable and can be persisted for `route explain`.
    pub tries_used: usize,

    /// True if the solution was found only on the last allowed try (`tries_used == max_offset_tries`).
    /// This makes "limited by max_offset_tries" provable.
    pub tries_exhausted: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct RouteOptions {
    pub clearance: f64,
    pub max_iters: usize,
    pub max_offset_tries: usize,
    pub offset_growth: f64,

    // scoring
    pub turn_weight: f64, // penalizza angoli stretti (gomiti)
    pub back_weight: f64, // penalizza "tornare indietro"

    // proximity scoring
    pub proximity_weight: f64, // intensità penalità
    pub proximity_margin: f64, // fascia extra oltre il raggio (warning band)
}

impl Default for RouteOptions {
    fn default() -> Self {
        Self {
            clearance: 0.2,
            max_iters: 32,
            max_offset_tries: 6,
            offset_growth: 1.4,
            turn_weight: 0.8, // taratura iniziale
            back_weight: 3.0, // più forte del turn
            proximity_weight: 1.5,
            proximity_margin: 0.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Route {
    pub waypoints: Vec<Point>,
    pub length: f64,
    pub iterations: usize,

    /// All detour choices made by the router (in chronological order)
    pub detours: Vec<DetourDecision>,
}

#[derive(Debug, Clone)]
pub struct CandidateScore {
    pub base: f64,      // lunghezza A->W->B
    pub turn: f64,      // penalità angolare
    pub back: f64,      // penalità backtracking
    pub proximity: f64, // penalità prossimità altri pianeti
}

impl CandidateScore {
    #[inline]
    pub fn total(&self) -> f64 {
        self.base + self.turn + self.back + self.proximity
    }
}

fn detour_candidates(a: Point, b: Point, hit: &Hit, offset: f64) -> Vec<Point> {
    let dir = normalize(sub(b, a));
    let n = perp(dir);

    let q = hit.closest.q;
    let c = hit.obstacle_center;
    let d = hit.closest.dist;

    // vettore "outward": dal centro verso Q
    let u = normalize(sub(q, c));

    // quanto serve per portarsi a r+clearance partendo da distanza d
    // (se d è già > r, questo vale ~0, ma hit in genere ha d < r)
    let push = (hit.obstacle_radius - d).max(0.0) + offset;

    let mut out = Vec::with_capacity(10);

    // 1) candidate "away from center" (molto forte)
    if !(u.x == 0.0 && u.y == 0.0) {
        out.push(add(q, mul(u, push)));
    }

    // 2) laterali classici
    out.push(add(q, mul(n, offset)));
    out.push(sub(q, mul(n, offset)));

    // 3) forward/back (utili nei cluster)
    out.push(add(q, mul(dir, offset)));
    out.push(sub(q, mul(dir, offset)));

    // 4) diagonali
    let d1 = normalize(add(dir, n));
    let d2 = normalize(sub(dir, n));
    out.push(add(q, mul(d1, offset)));
    out.push(add(q, mul(d2, offset)));

    out
}

fn evaluate_candidate(
    a: Point,
    w: Point,
    b: Point,
    obstacles: &[Obstacle],
    opts: RouteOptions,
    exclude_obstacle_id: Option<i64>,
) -> Option<CandidateScore> {
    if !is_segment_safe(a, w, obstacles) {
        return None;
    }
    if !is_segment_safe(w, b, obstacles) {
        return None;
    }

    // --- base length
    let base = dist(a, w) + dist(w, b);

    // --- turn penalty
    let u1 = normalize(sub(w, a));
    let u2 = normalize(sub(b, w));

    let mut turn = 0.0;
    if !(u1.x == 0.0 && u1.y == 0.0 || u2.x == 0.0 && u2.y == 0.0) {
        let cos_theta = clamp(dot(u1, u2), -1.0, 1.0);
        turn = opts.turn_weight * (1.0 - cos_theta);
    }

    // --- backtracking penalty
    let ab_dir = normalize(sub(b, a));
    let aw_dir = normalize(sub(w, a));

    let mut back = 0.0;
    if !(ab_dir.x == 0.0 && ab_dir.y == 0.0 || aw_dir.x == 0.0 && aw_dir.y == 0.0) {
        let progress = dot(ab_dir, aw_dir); // [-1..1]
        back = opts.back_weight * (-progress).max(0.0);
    }

    // --- proximity penalty
    let proximity = proximity_penalty_for_segment(
        a,
        w,
        obstacles,
        opts.proximity_margin,
        opts.proximity_weight,
        exclude_obstacle_id,
    ) + proximity_penalty_for_segment(
        w,
        b,
        obstacles,
        opts.proximity_margin,
        opts.proximity_weight,
        exclude_obstacle_id,
    );

    Some(CandidateScore {
        base,
        turn,
        back,
        proximity,
    })
}

pub fn compute_route(
    start: Point,
    end: Point,
    obstacles: &[Obstacle],
    opts: RouteOptions,
) -> Result<Route> {
    if start == end {
        return Ok(Route {
            waypoints: vec![start],
            length: 0.0,
            iterations: 0,
            detours: vec![],
        });
    }

    // Guardrails: avoid degenerate configs
    if opts.max_offset_tries == 0 {
        bail!("Invalid RouteOptions: max_offset_tries must be >= 1");
    }
    if opts.offset_growth <= 1.0 {
        bail!("Invalid RouteOptions: offset_growth must be > 1.0");
    }

    let mut waypoints = vec![start, end];
    let mut detours: Vec<DetourDecision> = Vec::new();
    let mut iterations = 0usize;

    while iterations < opts.max_iters {
        // 1) Find first colliding segment
        let mut first_collision: Option<(usize, Hit)> = None;

        for seg_idx in 0..(waypoints.len() - 1) {
            let a = waypoints[seg_idx];
            let b = waypoints[seg_idx + 1];

            if let Some(hit) = first_collision_on_segment(a, b, obstacles) {
                first_collision = Some((seg_idx, hit));
                break;
            }
        }

        // No collisions -> done
        if first_collision.is_none() {
            let length: f64 = waypoints.windows(2).map(|w| dist(w[0], w[1])).sum();
            return Ok(Route {
                waypoints,
                length,
                iterations,
                detours,
            });
        }

        let (seg_idx, hit) = first_collision.unwrap();
        let a = waypoints[seg_idx];
        let b = waypoints[seg_idx + 1];

        // 2) Find best detour candidate (with expanding offset)
        let base_offset = hit.obstacle_radius + opts.clearance;

        let mut best: Option<(Point, CandidateScore, f64, usize, bool)> = None;
        // (waypoint, score, offset_used, try_index, exhausted_at_selection)

        let mut offset = base_offset;
        let mut last_candidates: Vec<Point> = Vec::new();

        for try_idx in 0..opts.max_offset_tries {
            let candidates = detour_candidates(a, b, &hit, offset);
            last_candidates = candidates.clone();

            for w in candidates {
                let Some(score) =
                    evaluate_candidate(a, w, b, obstacles, opts, Some(hit.obstacle_id))
                else {
                    continue;
                };

                let better = match &best {
                    None => true,
                    Some((_bw, bs, _bo, _bi, _be)) => score.total() < bs.total(),
                };

                if better {
                    let exhausted = (try_idx + 1) == opts.max_offset_tries;
                    best = Some((w, score, offset, try_idx, exhausted));
                }
            }

            // As soon as we found a valid candidate at this offset,
            // we stop expanding offsets (keep the best among this offset's candidates).
            if best.is_some() {
                break;
            }

            offset *= opts.offset_growth;
        }

        let (detour_wp, detour_score, offset_used, try_idx, exhausted) = match best {
            Some(v) => v,
            None => {
                debug_failed_detour(a, b, &hit, &last_candidates, obstacles);
                bail!(
                    "No valid detour found for obstacle id={} (segment idx={})",
                    hit.obstacle_id,
                    seg_idx
                );
            }
        };

        // These two values make the "limited by max_offset_tries" statement provable.
        let tries_used = try_idx + 1;
        let tries_exhausted = exhausted;

        // 3) Record detour decision (before inserting)
        let obstacle_name = obstacles
            .iter()
            .find(|o| o.id == hit.obstacle_id)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "<unknown>".to_string());

        detours.push(DetourDecision {
            iteration: iterations,
            segment_index: seg_idx,

            obstacle_id: hit.obstacle_id,
            obstacle_name,
            obstacle_center: hit.obstacle_center,
            obstacle_radius: hit.obstacle_radius,

            closest_t: hit.closest.t,
            closest_q: hit.closest.q,
            closest_dist: hit.closest.dist,

            offset_used,
            waypoint: detour_wp,

            score: detour_score,

            tries_used,
            tries_exhausted,
        });

        // 4) Apply detour
        waypoints.insert(seg_idx + 1, detour_wp);
        iterations += 1;
    }

    bail!("Route computation exceeded max_iters={}", opts.max_iters)
}

fn proximity_penalty_for_segment(
    a: Point,
    b: Point,
    obstacles: &[Obstacle],
    margin: f64,
    weight: f64,
    exclude_id: Option<i64>,
) -> f64 {
    if margin <= 0.0 || weight <= 0.0 {
        return 0.0;
    }

    let mut pen = 0.0;

    for o in obstacles {
        if exclude_id == Some(o.id) {
            continue;
        }

        let cp = closest_point_on_segment(o.center, a, b);
        let warning = o.radius + margin;

        if cp.dist >= warning {
            continue;
        }

        if cp.dist <= o.radius {
            // should not happen if segment is already validated as safe;
            // keep it as a strong penalty to avoid accidental selection.
            pen += weight * 10.0;
            continue;
        }

        let x = (warning - cp.dist) / margin; // (0..1]
        pen += weight * x * x; // quadratic growth
    }

    pen
}
