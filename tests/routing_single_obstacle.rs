mod common;

use crate::common::assert_collision_free;
use sw_galaxy_map::routing::collision::Obstacle;
use sw_galaxy_map::routing::geometry::Point;
use sw_galaxy_map::routing::router::{RouteOptions, compute_route};

#[test]
fn route_with_single_obstacle_creates_detour() {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(10.0, 0.0);

    let obstacles = vec![Obstacle {
        id: 1,
        name: "Prakith".to_string(),
        center: Point::new(5.0, 0.0),
        radius: 0.6,
    }];

    let opts = RouteOptions {
        clearance: 0.03,
        ..Default::default()
    };

    let route = compute_route(start, end, &obstacles, opts).expect("route computation failed");

    // start -> W -> end
    assert!(route.waypoints.len() >= 3);

    assert_collision_free(&route, &obstacles);

    // sanity: no waypoint equals the obstacle center
    for w in &route.waypoints {
        assert_ne!(*w, obstacles[0].center);
    }
}
