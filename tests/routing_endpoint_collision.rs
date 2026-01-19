mod common;

use crate::common::assert_collision_free;
use sw_galaxy_map::routing::collision::{Obstacle, first_collision_on_segment};
use sw_galaxy_map::routing::geometry::Point;
use sw_galaxy_map::routing::router::{RouteOptions, compute_route};

#[test]
fn route_allows_destination_endpoint_collision() {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(10.0, 0.0);

    // Obstacle exactly at destination. With endpoint-safe logic, this must not prevent routing.
    let obstacles = vec![Obstacle {
        id: 99,
        center: end,
        radius: 2.0,
    }];

    let route = compute_route(start, end, &obstacles, RouteOptions::default())
        .expect("route computation failed");

    // Must succeed and preserve endpoints
    assert_eq!(route.waypoints.first().copied(), Some(start));
    assert_eq!(route.waypoints.last().copied(), Some(end));

    assert_collision_free(&route, &obstacles);

    // No "interior" collisions should remain
    for seg in route.waypoints.windows(2) {
        let a = seg[0];
        let b = seg[1];
        let hit = first_collision_on_segment(a, b, &obstacles);
        assert!(
            hit.is_none(),
            "Unexpected interior collision: {:?} on segment A={:?} B={:?}",
            hit,
            a,
            b
        );
    }
}
