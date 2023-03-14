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
            pub fn push(&mut self, value: MoveRecord<T>);
            pub fn pop(&mut self) -> Option<MoveRecord<T>>;
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

    pub fn get_linear(&self, key: f64) -> Option<(T, f64)>
    where
        T: LerpValue + Copy,
    {
        let p = self.0.partition_point(|rec| rec.key < key);
        if p > 0 && p < self.len() {
            let a = &self[p - 1];
            let b = &self[p];
            Some((a.value.lerp(b.value, invert_lerp(a.key, b.key, key)), 0.0))
        } else if !self.is_empty() {
            let a = &self[p.min(self.len() - 1)];
            Some((a.value, a.key - key))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MoveMode {
    Normal,
    Teleport,
}

#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
struct MoveSegment {
    pos_rec: MoveRecords<Vec2>,
    move_mode: MoveMode,
}

impl MoveSegment {
    fn new(start_pos: Vec2, distance: f64, move_mode: MoveMode) -> Self {
        Self {
            pos_rec: MoveRecords(vec![MoveRecord {
                key: distance,
                value: start_pos,
            }]),
            move_mode,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
pub struct SnakeHead {
    position: Vec2,
    time: f64,
    max_delay: f32,
    max_distance: f32,
    dis_rec: MoveRecords<f64>,
    segment_first: usize,
    segments: Vec<MoveSegment>,
}

impl SnakeHead {
    pub fn new(max_delay: f32, max_distance: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            time: 0.0,
            max_delay,
            max_distance,
            dis_rec: MoveRecords::new(),
            segment_first: 0,
            segments: Vec::new(),
        }
    }

    pub fn head_position(&self) -> Vec2 {
        self.position
    }

    pub fn move_head(&mut self, dt: f64, position: Vec2, move_mode: MoveMode) {
        self.position = position;
        self.time += dt;
        if !self.dis_rec.is_empty() {
            // move mode change, add new segment
            if move_mode != self.segments.last().unwrap().move_mode {
                let rec = self.segments.last().unwrap().pos_rec.last().unwrap();
                self.segments
                    .push(MoveSegment::new(rec.value, rec.key, move_mode));
            }

            // move back, remove position record
            let pos_rec = &mut self.segments.last_mut().unwrap().pos_rec;
            let last_dis = self.dis_rec.last().unwrap().value;
            let max_back = 40.0;
            let mut min_dis = f32::MAX;
            let mut index = usize::MAX;
            for i in (0..pos_rec.len() - 1).rev() {
                let p = &pos_rec[i];
                if p.key < last_dis - max_back {
                    break;
                }
                let dis = p.value.distance_squared(position);
                if dis < min_dis {
                    min_dis = dis;
                    index = i;
                }
            }
            if index < pos_rec.len() {
                let p = &pos_rec[index];
                if p.value.distance(position) as f64 + p.key < last_dis {
                    pos_rec.truncate(index + 1);
                }
            }

            // add record
            let last_pos = pos_rec.last().unwrap();
            let cur_dis = if move_mode == MoveMode::Normal {
                last_pos.key + last_pos.value.distance(position) as f64
            } else {
                last_pos.key
            };
            let max_dis = last_dis.max(cur_dis);
            if self.dis_rec.len() > 1
                && max_dis - self.dis_rec[self.dis_rec.len() - 2].value < 0.0001
            {
                // merge same distance record
                self.dis_rec.pop();
            }
            self.dis_rec.push(MoveRecord {
                key: self.time,
                value: max_dis,
            });
            let min_step = 5.0;
            if pos_rec.len() > 1
                && pos_rec[pos_rec.len() - 1].key - pos_rec[pos_rec.len() - 2].key < min_step
            {
                pos_rec.pop();
            }
            if position.distance_squared(pos_rec[pos_rec.len() - 1].value) >= 0.00000001 {
                pos_rec.push(MoveRecord {
                    key: cur_dis,
                    value: position,
                });
            }

            self.dis_rec.trim(self.max_delay as f64);
            self.segments[0].pos_rec.trim(self.max_distance as f64);
        } else {
            self.dis_rec.push(MoveRecord {
                key: self.time,
                value: 0.0,
            });
            self.segments
                .push(MoveSegment::new(position, 0.0, move_mode));
        }
    }

    fn foreach_pair<F: FnMut(usize, usize)>(len: usize, mut f: F) {
        for i in 0..len - 1 {
            for j in i + 1..len {
                f(i, j);
            }
        }
    }

    pub fn solve_body(
        &mut self,
        bodies: &mut [SnakeBody],
        max_move: f32,
        min_move: f32,
        radius: f32,
    ) {
        if self.dis_rec.is_empty() {
            return;
        }
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
        for body in bodies.iter_mut() {
            let time = self.time - body.delay as f64;
            let mut distance = self.dis_rec.get_linear(time).unwrap().0 - body.distance as f64;
            let mut iseg = body.segment.saturating_sub(self.segment_first);
            if distance >= self.segments[iseg].pos_rec.last().unwrap().key
                && iseg + 1 < self.segments.len()
                && self.segments[iseg + 1].move_mode == MoveMode::Teleport
            {
                iseg += 1;
                body.segment += 1;
                body.move_mode = MoveMode::Teleport;
            }
            if body.move_mode == MoveMode::Teleport {
                distance = distance.min(self.segments[iseg].pos_rec[0].key);
            }
            let (target, remain) = self.segments[iseg].pos_rec.get_linear(distance).unwrap();
            let current_distance = target.distance(body.position);
            let expect_distance = (current_distance - remain.max(0.0) as f32).max(0.0);
            let k = invert_lerp(1.5, 4.0, expect_distance / radius).clamp(0.0, 1.0);
            let max_move1 = max_move * (1.5 + k * 0.5);
            let delta = if current_distance > 0.0001 {
                (target - body.position) * (expect_distance.min(max_move1) / current_distance)
            } else {
                Vec2::ZERO
            };
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
                let d = if len > 0.0001 {
                    v0 * (radius / len - 0.5)
                } else {
                    let (x, y) = ((i * body_move.len() + j) as f32).sin_cos();
                    radius * Vec2::new(x, y)
                };
                if i > 0 {
                    body_move[i].position -= d;
                    body_move[j].position += d;
                } else {
                    body_move[j].position += d * 2.0;
                }
            });
            Self::foreach_pair(body_move.len(), |i, j| {
                let bm0 = &body_move[i];
                let bm1 = &body_move[j];
                if bm0.position.distance_squared(bm1.position) >= rr4 * 1.0001 {
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

        for i in 0..bodies.len() {
            if bodies[i].move_mode == MoveMode::Teleport {
                let iseg = bodies[i].segment.saturating_sub(self.segment_first);
                if iseg + 1 < self.segments.len() {
                    let pos = self.segments[iseg].pos_rec.last().unwrap().value;
                    if pos.distance_squared(head_pos) >= rr4 {
                        bodies[i].position = pos;
                        bodies[i].segment += 1;
                        bodies[i].move_mode = self.segments[iseg + 1].move_mode;
                    }
                }
            }
        }
        let seg_min = bodies.iter().map(|body| body.segment).min().unwrap();
        if seg_min > self.segment_first {
            self.segments.drain(0..seg_min - self.segment_first);
            self.segment_first = seg_min;
        }
    }

    pub fn get_path(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.segments
            .iter()
            .flat_map(|i| i.pos_rec.iter().skip(1).map(|r| r.value))
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SnakeBody {
    pub delay: f32,
    pub distance: f32,
    pub position: Vec2,
    pub target: Vec2,
    pub segment: usize,
    pub move_mode: MoveMode,
}

impl SnakeBody {
    pub fn new(delay: f32, distance: f32, position: Vec2) -> Self {
        Self {
            delay,
            distance,
            position,
            target: Vec2::ZERO,
            segment: 0,
            move_mode: MoveMode::Normal,
        }
    }
}
