use crate::routing::geometry::*;

#[derive(Debug, Clone)]
pub struct Obstacle {
    pub id: i64,
    pub name: String,
    pub center: Point,
    pub radius: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ClosestPoint {
    pub t: f64,
    pub q: Point,
    pub dist: f64,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub obstacle_id: i64,
    pub obstacle_center: Point,
    pub obstacle_radius: f64,
    pub closest: ClosestPoint,
}

pub fn closest_point_on_segment(p: Point, a: Point, b: Point) -> ClosestPoint {
    let ab = sub(b, a);
    let ap = sub(p, a);
    let ab2 = norm2(ab);

    let t = if ab2 == 0.0 {
        0.0
    } else {
        clamp(dot(ap, ab) / ab2, 0.0, 1.0)
    };

    let q = add(a, mul(ab, t));
    let d = dist(p, q);

    ClosestPoint { t, q, dist: d }
}

pub fn first_collision_on_segment(a: Point, b: Point, obstacles: &[Obstacle]) -> Option<Hit> {
    let mut best: Option<Hit> = None;

    for o in obstacles {
        let cp = closest_point_on_segment(o.center, a, b);

        // stessa regola: ignora endpoint collision
        const EPS_T: f64 = 1e-9;
        if cp.t <= EPS_T || cp.t >= 1.0 - EPS_T {
            continue;
        }

        if cp.dist < o.radius {
            let h = Hit {
                obstacle_id: o.id,
                obstacle_center: o.center,
                obstacle_radius: o.radius,
                closest: cp,
            };

            best = match best {
                None => Some(h),
                Some(prev) => {
                    let better = h.closest.t < prev.closest.t
                        || (h.closest.t == prev.closest.t && h.closest.dist < prev.closest.dist);
                    if better { Some(h) } else { Some(prev) }
                }
            };
        }
    }

    best
}

pub fn is_segment_safe(a: Point, b: Point, obstacles: &[Obstacle]) -> bool {
    for o in obstacles {
        if interior_collision_on_segment(a, b, o) {
            return false;
        }
    }
    true
}

pub fn proximity_penalty_for_segment(
    a: Point,
    b: Point,
    obstacles: &[Obstacle],
    margin: f64,
    weight: f64,
) -> f64 {
    if margin <= 0.0 || weight <= 0.0 {
        return 0.0;
    }

    let mut pen = 0.0;

    for o in obstacles {
        let cp = closest_point_on_segment(o.center, a, b);
        let warning = o.radius + margin;

        // collision handled elsewhere (d < o.radius)
        if cp.dist >= warning {
            continue;
        }
        if cp.dist <= o.radius {
            // should not happen here if segments are already validated as safe
            // keep it as strong penalty instead of panicking
            pen += weight * 10.0;
            continue;
        }

        // x in (0..1]
        let x = (warning - cp.dist) / margin;
        pen += weight * x * x;
    }

    pen
}

pub fn interior_collision_on_segment(a: Point, b: Point, o: &Obstacle) -> bool {
    const EPS_T: f64 = 1e-9;

    let cp = closest_point_on_segment(o.center, a, b);

    // collisioni solo su endpoint ammesse
    if cp.t <= EPS_T || cp.t >= 1.0 - EPS_T {
        return false;
    }

    cp.dist < o.radius
}
