#[cfg(feature = "bevy")]
use bevy::math::*;
#[cfg(feature = "glam")]
use glam::{Mat2, Vec2};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const SOLVE_STEP: i32 = 8;

#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
struct DistanceRecord {
    time: f64,
    distance: f64,
}

#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
struct PositionRecord {
    distance: f64,
    position: Vec2,
}

#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
pub struct SnakeHead {
    position: Vec2,
    time: f64,
    dis_rec: Vec<DistanceRecord>,
    pos_rec: Vec<PositionRecord>,
    max_delay: f32,
    max_distance: f32,
}

impl SnakeHead {
    pub fn new(max_delay: f32, max_distance: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            time: 0.0,
            dis_rec: Vec::new(),
            pos_rec: Vec::new(),
            max_delay,
            max_distance,
        }
    }

    pub fn reset(&mut self, position: Vec2) {
        self.position = position;
        self.time = 0.0;
        self.dis_rec.clear();
        self.pos_rec.clear();
        self.dis_rec.push(DistanceRecord {
            time: 0.0,
            distance: 0.0,
        });
        self.pos_rec.push(PositionRecord {
            distance: 0.0,
            position,
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
            let last_dis = self.dis_rec[n_dis - 1].distance;
            let max_back = 40.0;
            let mut min_dis = f32::MAX;
            let mut index = usize::MAX;
            for i in (0..n_pos - 1).rev() {
                let p = &self.pos_rec[i];
                if p.distance < last_dis - max_back {
                    break;
                }
                let dis = p.position.distance_squared(position);
                if dis < min_dis {
                    min_dis = dis;
                    index = i;
                }
            }
            if index < n_pos {
                let p = &self.pos_rec[index];
                if p.position.distance(position) as f64 + p.distance < last_dis {
                    self.pos_rec.truncate(index + 1);
                }
            }

            // add record
            let last_pos = self.pos_rec.last().unwrap();
            let delta = last_pos.position.distance(position);
            let new_dis = last_dis.max(last_pos.distance + delta as f64);
            if n_dis > 1 && new_dis - self.dis_rec[n_dis - 2].distance < 0.0001 {
                // merge same distance record
                self.dis_rec[n_dis - 1].time = self.time;
            } else {
                self.dis_rec.push(DistanceRecord {
                    time: self.time,
                    distance: new_dis,
                });
            }
            let min_step = 5.0;
            if delta >= min_step {
                let distance = last_pos.distance + delta as f64;
                self.pos_rec.push(PositionRecord { distance, position });
            }
            if self.dis_rec.len() > 128
                && self.dis_rec[128].time + (self.max_delay as f64)
                    < self.dis_rec.last().unwrap().time
            {
                self.dis_rec.drain(..128);
            }
            if self.pos_rec.len() > 128
                && self.pos_rec[128].distance + (self.max_distance as f64)
                    < self.pos_rec.last().unwrap().distance
            {
                self.pos_rec.drain(..128);
            }
        } else {
            self.reset(position);
        }
    }

    pub fn get_distance(&self, time: f64) -> f64 {
        let p = match self
            .dis_rec
            .binary_search_by(|d| d.time.partial_cmp(&time).unwrap())
        {
            Ok(p) => p,
            Err(p) => p,
        };
        if p > 0 && p < self.dis_rec.len() {
            let dis_rec0 = &self.dis_rec[p - 1];
            let dis_rec1 = &self.dis_rec[p];
            let k = (time - dis_rec0.time) / (dis_rec1.time - dis_rec0.time);
            k * (dis_rec1.distance - dis_rec0.distance) + dis_rec0.distance
        } else if !self.dis_rec.is_empty() {
            let p1 = p.min(self.dis_rec.len() - 1);
            self.dis_rec[p1].distance
        } else {
            0.0
        }
    }

    pub fn get_position(&self, distance: f64) -> (Vec2, f32) {
        let p = match self
            .pos_rec
            .binary_search_by(|p| p.distance.partial_cmp(&distance).unwrap())
        {
            Ok(p) => p,
            Err(p) => p,
        };
        if p > 0 && p < self.pos_rec.len() {
            let pos_rec0 = &self.pos_rec[p - 1];
            let pos_rec1 = &self.pos_rec[p];
            let k = (distance - pos_rec0.distance) / (pos_rec1.distance - pos_rec0.distance);
            (pos_rec0.position.lerp(pos_rec1.position, k as f32), 0.0)
        } else if !self.pos_rec.is_empty() {
            let p1 = p.min(self.pos_rec.len() - 1);
            (
                self.pos_rec[p1].position,
                (self.pos_rec[p1].distance - distance) as f32,
            )
        } else {
            (Vec2::ZERO, 0.0)
        }
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

    pub fn solve_body(
        &self,
        bodies: &mut [SnakeBody],
        max_move: f32,
        min_move: f32,
        radius: f32,
    ) -> Vec<Vec2> {
        let head_pos = self.position;
        let rr4 = radius * radius * 4.0;

        struct BodyMove {
            position: Vec2,
            delta: Vec2,
            origin: Vec2,
            target: Vec2,
            max_move: f32,
        }
        let mut body_move: Vec<_> = bodies
            .iter()
            .map(|body| {
                let time = self.time - body.delay as f64;
                let distance = self.get_distance(time);
                let (target, stop_distance) = self.get_position(distance - body.distance as f64);
                let current_distance = target.distance(body.position);
                let remain_distance = (current_distance - stop_distance).max(0.0);
                let k = ((remain_distance / radius - 1.5) / 2.5).clamp(0.0, 1.0);
                let max_move1 = max_move * (k * 0.5 + 1.5);
                let delta =
                    (target - body.position) * (remain_distance.min(max_move1) / current_distance);
                BodyMove {
                    position: body.position,
                    delta,
                    origin: body.position,
                    target,
                    max_move: max_move1,
                }
            })
            .collect();

        for _ in 0..SOLVE_STEP {
            for bm in body_move.iter_mut() {
                bm.position += bm.delta / SOLVE_STEP as f32;
            }
            for i in 0..body_move.len() {
                if body_move[i].position.distance_squared(head_pos) < rr4 {
                    let bm0 = &body_move[i];
                    let v0 = bm0.position - head_pos;
                    let len = v0.length();
                    if len > 0.0001 {
                        body_move[i].position += v0 * (radius * 2.0 / len - 1.0);
                    }
                }
                for j in i + 1..body_move.len() {
                    let bm0 = &body_move[i];
                    let bm1 = &body_move[j];
                    if bm0.position.distance_squared(bm1.position) < rr4 {
                        let v0 = bm1.position - bm0.position;
                        let len = v0.length();
                        if len > 0.0001 {
                            let d = v0 * (radius / len - 0.5);
                            body_move[i].position -= d;
                            body_move[j].position += d;
                        }
                    }
                }
            }
            for i in 0..body_move.len() {
                if body_move[i].position.distance_squared(head_pos) < rr4 * 1.0001 {
                    let bm0 = &body_move[i];
                    let detour = Self::detour(
                        bm0.position,
                        bm0.delta,
                        bm0.target,
                        head_pos,
                        Vec2::ZERO,
                        head_pos,
                    );
                    if detour.length_squared() > 0.0001 {
                        let pos = bm0.position + 2.0 * detour;
                        if body_move
                            .iter()
                            .enumerate()
                            .filter(|(j, _)| *j != i)
                            .all(|(_, bm)| pos.distance_squared(bm.position) >= rr4)
                        {
                            body_move[i].position = pos;
                        }
                    }
                }
                for j in i + 1..body_move.len() {
                    if body_move[i]
                        .position
                        .distance_squared(body_move[j].position)
                        < rr4
                    {
                        let bm0 = &body_move[i];
                        let bm1 = &body_move[j];
                        let detour = Self::detour(
                            bm0.position,
                            bm0.delta,
                            bm0.target,
                            bm1.position,
                            bm1.delta,
                            bm1.target,
                        );
                        if detour.length_squared() > 0.0001 {
                            let pos0 = bm0.position + detour;
                            let pos1 = bm1.position - detour;
                            if pos0.distance_squared(head_pos) >= rr4
                                && pos1.distance_squared(head_pos) >= rr4
                                && body_move
                                    .iter()
                                    .enumerate()
                                    .filter(|(k, _)| *k != i && *k != j)
                                    .all(|(_, bm)| {
                                        pos0.distance_squared(bm.position) >= rr4
                                            && pos1.distance_squared(bm.position) >= rr4
                                    })
                            {
                                body_move[i].position = pos0;
                                body_move[j].position = pos1;
                            }
                        }
                    }
                }
            }
            for bm in body_move.iter_mut() {
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

        for (body, body_move) in bodies.iter_mut().zip(body_move.iter()) {
            body.position = body_move.position;
        }
        body_move.iter().map(|bm| bm.target).collect()
    }

    pub fn get_path(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.pos_rec.iter().map(|r| r.position)
    }
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SnakeBody {
    pub delay: f32,
    pub distance: f32,
    pub position: Vec2,
}

impl SnakeBody {
    pub fn new(delay: f32, distance: f32, position: Vec2) -> Self {
        Self {
            delay,
            distance,
            position,
        }
    }
}
