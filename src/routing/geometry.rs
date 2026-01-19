#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

pub fn add(a: Point, b: Point) -> Point {
    Point::new(a.x + b.x, a.y + b.y)
}
pub fn sub(a: Point, b: Point) -> Point {
    Point::new(a.x - b.x, a.y - b.y)
}
pub fn mul(a: Point, k: f64) -> Point {
    Point::new(a.x * k, a.y * k)
}
pub fn dot(a: Point, b: Point) -> f64 {
    a.x * b.x + a.y * b.y
}
pub fn norm2(a: Point) -> f64 {
    dot(a, a)
}
pub fn norm(a: Point) -> f64 {
    norm2(a).sqrt()
}
pub fn dist(a: Point, b: Point) -> f64 {
    norm(sub(a, b))
}
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}
pub fn normalize(v: Point) -> Point {
    let n = norm(v);
    if n == 0.0 {
        Point::new(0.0, 0.0)
    } else {
        Point::new(v.x / n, v.y / n)
    }
}
pub fn perp(v: Point) -> Point {
    Point::new(-v.y, v.x)
}
