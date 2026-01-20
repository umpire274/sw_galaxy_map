mod common;

use crate::common::assert_collision_free;
use sw_galaxy_map::routing::collision::Obstacle;
use sw_galaxy_map::routing::geometry::Point;
use sw_galaxy_map::routing::router::{RouteOptions, compute_route};

#[test]
fn route_with_multiple_obstacles_is_safe() {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(12.0, 0.0);

    let obstacles = vec![
        Obstacle {
            id: 1,
            name: "Prakith".to_string(),
            center: Point::new(4.0, 0.0),
            radius: 0.6,
        },
        Obstacle {
            id: 2,
            name: "Keeara Major".to_string(),
            center: Point::new(8.0, 0.0),
            radius: 0.6,
        },
    ];

    let opts = RouteOptions {
        clearance: 0.025,
        max_iters: 10,
        ..Default::default()
    };

    let route = compute_route(start, end, &obstacles, opts).expect("route computation failed");

    assert!(route.waypoints.len() >= 3);
    assert!(route.iterations > 0);

    assert_collision_free(&route, &obstacles);

    // Basic monotonicity: start and end preserved
    assert_eq!(route.waypoints.first().copied(), Some(start));
    assert_eq!(route.waypoints.last().copied(), Some(end));
}
