use super::snake_move::{self, SnakeHead};
use glam::Vec2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for Vector2 {
    fn from(v: Vec2) -> Self {
        Self {
            x: v.x,
            y: v.y,
        }
    }
}

impl From<Vector2> for Vec2 {
    fn from(v: Vector2) -> Self {
        Self {
            x: v.x,
            y: v.y,
        }
    }
}

#[repr(C)]
pub struct SnakeBody {
    pub delay: f32,
    pub distance: f32,
    pub position: Vector2,
    pub target: Vector2,
}

#[no_mangle]
pub extern "C" fn snake_new(max_delay: f32, max_distance: f32) -> *mut SnakeHead {
    Box::into_raw(Box::new(SnakeHead::new(max_delay, max_distance)))
}

#[no_mangle]
pub extern "C" fn snake_drop(head: *mut SnakeHead) {
    unsafe { Box::from_raw(head) };
}

#[no_mangle]
pub extern "C" fn snake_reset(head: &mut SnakeHead, position: Vector2) {
    head.reset(position.into());
}

#[no_mangle]
pub extern "C" fn snake_move_head(head: &mut SnakeHead, position: Vector2, dt: f64) {
    head.move_head(position.into(), dt);
}

#[no_mangle]
pub extern "C" fn snake_solve_body(
    head: &mut SnakeHead,
    bodies: *mut SnakeBody,
    num_bodies: usize,
    max_move: f32,
    min_move: f32,
    radius: f32,
) {
    let bodies0 = unsafe { std::slice::from_raw_parts_mut(bodies, num_bodies) };
    let mut bodies1: Vec<_> = bodies0
        .iter()
        .map(|b| snake_move::SnakeBody {
            delay: b.delay,
            distance: b.distance,
            position: b.position.into(),
            target: b.target.into(),
        })
        .collect();
    head.solve_body(&mut bodies1, max_move, min_move, radius);
    for (b0, b1) in bodies0.iter_mut().zip(bodies1.iter()) {
        b0.position = b1.position.into();
        b0.target = b1.target.into();
    }
}
