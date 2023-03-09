#[cfg(feature = "bevy")]
use bevy::math::*;
use delegate::delegate;
#[cfg(feature = "glam")]
use glam::{Mat2, Vec2};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

const SOLVE_STEP: i32 = 8;

fn invert_lerp<T: num_traits::Float>(min: T, max: T, k: T) -> T {
    (k - min) / (max - min)
}

trait LerpValue {
    fn lerp(self, other: Self, k: f64) -> Self;
}

impl LerpValue for f64 {
    fn lerp(self, other: Self, k: f64) -> Self {
        self + (other - self) * k
    }
}

impl LerpValue for Vec2 {
    fn lerp(self, other: Self, k: f64) -> Self {
        Vec2::lerp(self, other, k as f32)
    }
}

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
struct MoveRecord<T> {
    key: f64,
    value: T,
}

#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
struct MoveRecords<T>(Vec<MoveRecord<T>>);

impl<T> Index<usize> for MoveRecords<T> {
    type Output = MoveRecord<T>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> IndexMut<usize> for MoveRecords<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<T> MoveRecords<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    delegate! {
        to self.0 {
            pub fn len(&self) -> usize;
            pub fn is_empty(&self) -> bool;
            pub fn clear(&mut self);
            pub fn push(&mut self, value: MoveRecord<T>);
            pub fn last(&self) -> Option<&MoveRecord<T>>;
            pub fn truncate(&mut self, len: usize);
            pub fn iter(&self) -> std::slice::Iter<'_, MoveRecord<T>>;
        }
    }

    pub fn trim(&mut self, threshold: f64) {
        if self.len() > 128 && self[128].key + threshold < self.last().unwrap().key {
            self.0.drain(..128);
        }
    }

    pub fn get_linear(&self, key: f64) -> (T, f64)
    where
        T: LerpValue + Default + Copy,
    {
        let p = self.0.partition_point(|rec| rec.key < key);
        if p > 0 && p < self.len() {
            let a = &self[p - 1];
            let b = &self[p];
            (a.value.lerp(b.value, invert_lerp(a.key, b.key, key)), 0.0)
        } else if !self.is_empty() {
            let a = &self[p.min(self.len() - 1)];
            (a.value, a.key - key)
        } else {
            (T::default(), 0.0)
        }
    }
}

#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
pub struct SnakeHead {
    position: Vec2,
    time: f64,
    dis_rec: MoveRecords<f64>,
    pos_rec: MoveRecords<Vec2>,
    max_delay: f32,
    max_distance: f32,
}

impl SnakeHead {
    pub fn new(max_delay: f32, max_distance: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            time: 0.0,
            dis_rec: MoveRecords::new(),
            pos_rec: MoveRecords::new(),
            max_delay,
            max_distance,
        }
    }

    pub fn reset(&mut self, position: Vec2) {
        self.position = position;
        self.time = 0.0;
        self.dis_rec.clear();
        self.pos_rec.clear();
        self.dis_rec.push(MoveRecord {
            key: 0.0,
            value: 0.0,
        });
        self.pos_rec.push(MoveRecord {
            key: 0.0,
            value: position,
        });
    }

    pub fn head_position(&self) -> Vec2 {
        self.position
    }

    pub fn move_head(&mut self, position: Vec2, dt: f64) {
        self.position = position;
        self.time += dt;
        let n_dis = self.dis_rec.len();
        let n_pos = self.pos_rec.len();
        if n_dis > 0 && n_pos > 0 {
            // move back, remove position record
            let last_dis = self.dis_rec[n_dis - 1].value;
            let max_back = 40.0;
            let mut min_dis = f32::MAX;
            let mut index = usize::MAX;
            for i in (0..n_pos - 1).rev() {
                let p = &self.pos_rec[i];
                if p.key < last_dis - max_back {
                    break;
                }
                let dis = p.value.distance_squared(position);
                if dis < min_dis {
                    min_dis = dis;
                    index = i;
                }
            }
            if index < n_pos {
                let p = &self.pos_rec[index];
                if p.value.distance(position) as f64 + p.key < last_dis {
                    self.pos_rec.truncate(index + 1);
                }
            }

            // add record
            let last_pos = self.pos_rec.last().unwrap();
            let delta = last_pos.value.distance(position);
            let new_dis = last_dis.max(last_pos.key + delta as f64);
            if n_dis > 1 && new_dis - self.dis_rec[n_dis - 2].value < 0.0001 {
                // merge same distance record
                self.dis_rec[n_dis - 1].key = self.time;
            } else {
                self.dis_rec.push(MoveRecord {
                    key: self.time,
                    value: new_dis,
                });
            }
            let min_step = 5.0;
            if delta >= min_step {
                let distance = last_pos.key + delta as f64;
                self.pos_rec.push(MoveRecord {
                    key: distance,
                    value: position,
                });
            }
            self.dis_rec.trim(self.max_delay as f64);
            self.pos_rec.trim(self.max_distance as f64);
        } else {
            self.reset(position);
        }
    }

    pub fn get_distance(&self, time: f64) -> f64 {
        self.dis_rec.get_linear(time).0
    }

    pub fn get_position(&self, distance: f64) -> (Vec2, f64) {
        self.pos_rec.get_linear(distance)
    }

    pub fn detour(p0: Vec2, v0: Vec2, t0: Vec2, p1: Vec2, v1: Vec2, t1: Vec2) -> Vec2 {
        let dp = p1 - p0;
        if dp.dot(t1 - t0) < -0.001 {
            let mut angle = 4.0 / SOLVE_STEP as f32;
            let vertical = Vec2::new(dp.y, -dp.x);
            if v0.dot(vertical) < v1.dot(vertical) {
                angle = -angle;
            }
            dp - Mat2::from_angle(angle).mul_vec2(dp)
        } else {
            Vec2::ZERO
        }
    }

    fn foreach_pair<F: FnMut(usize, usize)>(len: usize, mut f: F) {
        for i in 0..len - 1 {
            for j in i + 1..len {
                f(i, j);
            }
        }
    }

    pub fn solve_body(&self, bodies: &mut [SnakeBody], max_move: f32, min_move: f32, radius: f32) {
        let head_pos = self.position;
        let rr4 = radius * radius * 4.0;

        struct BodyMove {
            position: Vec2,
            delta: Vec2,
            origin: Vec2,
            target: Vec2,
            max_move: f32,
        }
        let mut body_move: Vec<BodyMove> = Vec::with_capacity(bodies.len() + 1);
        body_move.push(BodyMove {
            position: head_pos,
            delta: Vec2::ZERO,
            origin: head_pos,
            target: head_pos,
            max_move: 0.0,
        });
        for body in bodies.iter() {
            let time = self.time - body.delay as f64;
            let distance = self.get_distance(time);
            let (target, stop_distance) = self.get_position(distance - body.distance as f64);
            let current_distance = target.distance(body.position);
            let remain_distance = (current_distance - stop_distance as f32).max(0.0);
            let k = invert_lerp(1.5, 4.0, remain_distance / radius).clamp(0.0, 1.0);
            let max_move1 = max_move * (1.5 + k * 0.5);
            let delta =
                (target - body.position) * (remain_distance.min(max_move1) / current_distance);
            body_move.push(BodyMove {
                position: body.position,
                delta,
                origin: body.position,
                target,
                max_move: max_move1,
            });
        }

        for _ in 0..SOLVE_STEP {
            for bm in body_move.iter_mut().skip(1) {
                bm.position += bm.delta / SOLVE_STEP as f32;
            }
            Self::foreach_pair(body_move.len(), |i, j| {
                let bm0 = &body_move[i];
                let bm1 = &body_move[j];
                if bm0.position.distance_squared(bm1.position) >= rr4 {
                    return;
                }
                let v0 = bm1.position - bm0.position;
                let len = v0.length();
                if len > 0.0001 {
                    let d = v0 * (radius / len - 0.5);
                    if i > 0 {
                        body_move[i].position -= d;
                        body_move[j].position += d;
                    } else {
                        body_move[j].position += d * 2.0;
                    }
                }
            });
            Self::foreach_pair(body_move.len(), |i, j| {
                let bm0 = &body_move[i];
                let bm1 = &body_move[j];
                if bm0.position.distance_squared(body_move[j].position) >= rr4 * 1.0001 {
                    return;
                }
                let dp = bm1.position - bm0.position;
                if dp.dot(bm1.target - bm0.target) >= -0.001 {
                    return;
                }
                let vertical = Vec2::new(dp.y, -dp.x);
                let mut angle = 4.0 / SOLVE_STEP as f32;
                if bm0.delta.dot(vertical) < bm1.delta.dot(vertical) {
                    angle = -angle;
                }
                let offset = Mat2::from_angle(angle).mul_vec2(dp) - dp;
                let check_move = |pos: Vec2| {
                    for (k, bm) in body_move.iter().enumerate() {
                        if k != i && k != j && pos.distance_squared(bm.position) < rr4 {
                            return false;
                        }
                    }
                    true
                };
                if i > 0 {
                    let pos0 = bm0.position - offset;
                    let pos1 = bm1.position + offset;
                    if check_move(pos0) && check_move(pos1) {
                        body_move[i].position = pos0;
                        body_move[j].position = pos1;
                    }
                } else {
                    let pos = bm1.position + 2.0 * offset;
                    if check_move(pos) {
                        body_move[j].position = pos;
                    }
                }
            });
            for bm in body_move.iter_mut().skip(1) {
                let distance = bm.origin.distance(bm.position);
                if distance >= min_move / SOLVE_STEP as f32 {
                    if distance > bm.max_move {
                        bm.position = bm.origin.lerp(bm.position, bm.max_move / distance);
                    }
                } else {
                    bm.position = bm.origin;
                }
            }
        }

        for (body, bm) in bodies.iter_mut().zip(body_move.iter().skip(1)) {
            body.position = bm.position;
            body.target = bm.target;
        }
    }

    pub fn get_path(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.pos_rec.iter().map(|r| r.value)
    }
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SnakeBody {
    pub delay: f32,
    pub distance: f32,
    pub position: Vec2,
    pub target: Vec2,
}

impl SnakeBody {
    pub fn new(delay: f32, distance: f32, position: Vec2) -> Self {
        Self {
            delay,
            distance,
            position,
            target: Vec2::ZERO,
        }
    }
}
