use delegate::delegate;
use glam::{Mat2, Vec2, Vec3, Vec3Swizzles};
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

impl LerpValue for Vec3 {
    fn lerp(self, other: Self, k: f64) -> Self {
        Vec3::lerp(self, other, k as f32)
    }
}

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
struct MoveRecord<T> {
    key: f64,
    value: T,
}

impl MoveRecord<Vec3> {
    fn pos2d(&self) -> Vec2 {
        self.value.xy()
    }
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

    pub fn trim(&mut self, key: f64) {
        let len = self.len();
        if len > 2 && self[len / 2 + 1].key < key {
            self.0.drain(..len / 2);
        }
    }

    pub fn get_linear(&self, key: f64) -> Option<T>
    where
        T: LerpValue + Copy,
    {
        let p = self.0.partition_point(|rec| rec.key < key);
        if p > 0 && p < self.len() {
            let a = &self[p - 1];
            let b = &self[p];
            if a.key + 0.0001 < b.key {
                Some(a.value.lerp(b.value, invert_lerp(a.key, b.key, key)))
            } else {
                Some(b.value)
            }
        } else if !self.is_empty() {
            if p == 0 {
                Some(self[0].value)
            } else {
                Some(self[self.len() - 1].value)
            }
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
pub struct SnakeHead {
    position: Vec3,
    time: f64,
    dis_rec: MoveRecords<f64>,
    pos_rec: MoveRecords<Vec3>,
    mode_rec: MoveRecords<(MoveMode, Vec3)>,
}

impl SnakeHead {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            time: 0.0,
            dis_rec: MoveRecords::new(),
            pos_rec: MoveRecords::new(),
            mode_rec: MoveRecords::new(),
            // segment_first: 0,
            // segments: Vec::new(),
        }
    }

    pub fn head_position(&self) -> Vec3 {
        self.position
    }

    pub fn move_head(&mut self, dt: f64, position: Vec3, move_mode: MoveMode) {
        self.position = position;
        self.time += dt;
        if !self.dis_rec.is_empty() {
            let pos2d = position.xy();
            let new_seg = match move_mode {
                MoveMode::Teleport => true,
                _ => move_mode != self.mode_rec.last().unwrap().value.0,
            };
            if new_seg {
                let pos1 = match move_mode {
                    MoveMode::Teleport => position,
                    _ => self.pos_rec.last().unwrap().value,
                };
                self.mode_rec.push(MoveRecord {
                    key: self.pos_rec.last().unwrap().key,
                    value: (move_mode, pos1),
                });
            }

            // move back, remove position record
            let pos_rec = &mut self.pos_rec;
            let last_dis = self.dis_rec.last().unwrap().value;
            let back_limit = (last_dis - 40.0).max(self.mode_rec.last().unwrap().key);
            let mut min_dis = f32::MAX;
            let mut index = usize::MAX;
            for i in (0..pos_rec.len() - 1).rev() {
                let p = &pos_rec[i];
                if p.key <= back_limit {
                    break;
                }
                let dis = p.pos2d().distance_squared(pos2d);
                if dis < min_dis {
                    min_dis = dis;
                    index = i;
                }
            }
            if index < pos_rec.len() {
                let p = &pos_rec[index];
                if p.pos2d().distance(pos2d) as f64 + p.key < last_dis {
                    pos_rec.truncate(index + 1);
                }
            }

            // add record
            let last_pos = pos_rec.last().unwrap();
            let cur_dis = match move_mode {
                MoveMode::Teleport => last_pos.key,
                _ => last_pos.key + last_pos.pos2d().distance(pos2d) as f64,
            };
            let max_dis = last_dis.max(cur_dis);
            if !new_seg
                && self.dis_rec.len() > 1
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
            if !new_seg
                && pos_rec.len() > 1
                && pos_rec[pos_rec.len() - 1].key - pos_rec[pos_rec.len() - 2].key < min_step
            {
                pos_rec.pop();
            }
            if new_seg || pos2d.distance_squared(pos_rec[pos_rec.len() - 1].pos2d()) >= 0.00000001 {
                pos_rec.push(MoveRecord {
                    key: cur_dis,
                    value: position,
                });
            }
        } else {
            self.dis_rec.push(MoveRecord {
                key: self.time,
                value: 0.0,
            });
            self.pos_rec.push(MoveRecord {
                key: 0.0,
                value: position,
            });
            self.mode_rec.push(MoveRecord {
                key: 0.0,
                value: (move_mode, position),
            });
        }
    }

    pub fn update_body(&mut self, bodies: &mut [SnakeBody], radius: f32) {
        if self.dis_rec.is_empty() {
            return;
        }
        let head_pos = self.position.xy();
        let rr4 = radius * radius * 4.0;
        let mut min_time = f64::MAX;
        let mut min_distance = f64::MAX;
        let mut min_segment = usize::MAX;
        for i in 0..bodies.len() {
            let time = self.time - bodies[i].delay as f64;
            min_time = min_time.min(time);
            let distance = self.dis_rec.get_linear(time).unwrap() - bodies[i].distance as f64;
            min_distance = min_distance.min(distance);
            let iseg = bodies[i].segment;
            if iseg + 1 < self.mode_rec.len() {
                let can_leave = match self.mode_rec[iseg].value.0 {
                    MoveMode::Normal => distance >= self.mode_rec[iseg + 1].key,
                    MoveMode::Teleport => true,
                };
                if can_leave {
                    match self.mode_rec[iseg + 1].value.0 {
                        MoveMode::Normal => {
                            bodies[i].segment += 1;
                        }
                        MoveMode::Teleport => {
                            let pos = self.mode_rec[iseg + 1].value.1;
                            let pos2d = pos.xy();
                            if pos2d.distance_squared(head_pos) >= rr4
                                && bodies[..i]
                                    .iter()
                                    .all(|body| pos2d.distance_squared(body.position.xy()) >= rr4)
                            {
                                bodies[i].position = pos;
                                bodies[i].segment += 1;
                            }
                        }
                    };
                }
                min_segment = min_segment.min(bodies[i].segment);
            }
            match self.mode_rec[iseg].value.0 {
                MoveMode::Normal => {
                    bodies[i].target = if distance > self.mode_rec[iseg].key {
                        self.pos_rec.get_linear(distance).unwrap()
                    } else {
                        let p0 = self.mode_rec[iseg].value.1;
                        let remain = (self.mode_rec[iseg].key - distance) as f32;
                        let dis = p0.xy().distance(bodies[i].position.xy());
                        if dis > remain {
                            p0.lerp(bodies[i].position, remain / dis)
                        } else {
                            bodies[i].position
                        }
                    };
                }
                MoveMode::Teleport => {
                    bodies[i].target = bodies[i].position;
                }
            }
        }
        if min_segment > 0 && min_segment < self.mode_rec.len() {
            self.mode_rec.0.drain(0..min_segment);
            for body in bodies.iter_mut() {
                body.segment -= min_segment;
            }
        }
        self.dis_rec.trim(min_time);
        self.pos_rec.trim(min_distance);
    }

    fn foreach_pair<F: FnMut(usize, usize)>(len: usize, mut f: F) {
        for i in 0..len - 1 {
            for j in i + 1..len {
                f(i, j);
            }
        }
    }

    pub fn solve_body<F>(
        &self,
        bodies: &mut [SnakeBody],
        max_move: f32,
        min_move: f32,
        radius: f32,
        fix_position: Option<F>,
    ) where
        F: Fn(&SnakeBody, Vec3, Vec3) -> Vec3,
    {
        let rr4 = radius * radius * 4.0;

        struct BodyMove {
            position: Vec3,
            delta: Vec2,
            target: Vec3,
            max_move: f32,
            fix_offset: Vec2,
        }
        impl BodyMove {
            fn pos2d(&self) -> Vec2 {
                self.position.xy()
            }
            fn set_pos2d(&mut self, p: Vec2) {
                self.position.x = p.x;
                self.position.y = p.y;
            }
            fn add_pos2d(&mut self, v: Vec2) {
                self.position.x += v.x;
                self.position.y += v.y;
            }
        }
        let mut body_move: Vec<BodyMove> = Vec::with_capacity(bodies.len() + 1);
        body_move.push(BodyMove {
            position: self.position,
            delta: Vec2::ZERO,
            target: self.position,
            max_move: 0.0,
            fix_offset: Vec2::ZERO,
        });
        for body in bodies.iter() {
            let mut max_move1 = max_move;
            let mut delta = Vec2::ZERO;
            let target_distance = body.target.xy().distance(body.position.xy());
            if target_distance > 0.0001 {
                let k: f32 = invert_lerp(1.5, 4.0, target_distance / radius).clamp(0.0, 1.0);
                max_move1 = max_move * (1.5 + k * 0.5);
                delta = body.target.xy() - body.position.xy();
                if target_distance > max_move1 {
                    delta *= max_move1 / target_distance;
                }
            }
            body_move.push(BodyMove {
                position: body.position,
                delta,
                target: body.target,
                max_move: max_move1,
                fix_offset: Vec2::ZERO,
            });
        }

        for _ in 0..SOLVE_STEP {
            for bm in body_move.iter_mut().skip(1) {
                bm.add_pos2d(bm.delta / SOLVE_STEP as f32);
            }
            Self::foreach_pair(body_move.len(), |i, j| {
                let bm0 = &body_move[i];
                let bm1 = &body_move[j];
                if (bm0.position.z - bm1.position.z).abs() > radius * 2.0 {
                    return;
                }
                if bm0.pos2d().distance_squared(bm1.pos2d()) >= rr4 {
                    return;
                }
                let v0 = bm1.pos2d() - bm0.pos2d();
                let len = v0.length();
                let d = if len > 0.0001 {
                    v0 * (radius / len - 0.5)
                } else {
                    let (x, y) = ((i * body_move.len() + j) as f32).sin_cos();
                    radius * Vec2::new(x, y)
                };
                if i > 0 {
                    body_move[i].add_pos2d(-d);
                    body_move[j].add_pos2d(d);
                } else {
                    body_move[j].add_pos2d(d * 2.0);
                }
            });
            Self::foreach_pair(body_move.len(), |i, j| {
                let bm0 = &body_move[i];
                let bm1 = &body_move[j];
                if (bm0.position.z - bm1.position.z).abs() > radius * 2.0 {
                    return;
                }
                if bm0.pos2d().distance_squared(bm1.pos2d()) >= rr4 * 1.0001 {
                    return;
                }
                let dp = bm1.pos2d() - bm0.pos2d();
                if dp.dot(bm1.target.xy() - bm0.target.xy()) >= -0.001 {
                    return;
                }
                let vertical = Vec2::new(dp.y, -dp.x);
                let mut angle = 24.0 * max_move / radius / SOLVE_STEP as f32;
                let rand_offset = (bm0.position + bm1.position)
                    .as_ref()
                    .iter()
                    .sum::<f32>()
                    .sin_cos();
                if (bm0.delta - bm1.delta + Vec2::new(rand_offset.0, rand_offset.1)).dot(vertical)
                    < 0.0
                {
                    angle = -angle;
                }
                let offset = Mat2::from_angle(angle).mul_vec2(dp) - dp;
                if bm0.fix_offset.dot(offset) > 0.0 || bm1.fix_offset.dot(offset) < 0.0 {
                    return;
                }
                let check_move = |pos: Vec2| {
                    for (k, bm) in body_move.iter().enumerate() {
                        if k != i && k != j && pos.distance_squared(bm.pos2d()) < rr4 {
                            return false;
                        }
                    }
                    true
                };
                if i > 0 {
                    let pos0 = bm0.pos2d() - offset;
                    let pos1 = bm1.pos2d() + offset;
                    if check_move(pos0) && check_move(pos1) {
                        body_move[i].set_pos2d(pos0);
                        body_move[j].set_pos2d(pos1);
                    }
                } else {
                    let pos = bm1.pos2d() + 2.0 * offset;
                    if check_move(pos) {
                        body_move[j].set_pos2d(pos);
                    }
                }
            });
            for (bm, body) in body_move.iter_mut().skip(1).zip(bodies.iter()) {
                let origin = body.position.xy();
                let distance = origin.distance(bm.pos2d());
                bm.fix_offset = Vec2::ZERO;
                if distance >= min_move / SOLVE_STEP as f32 {
                    if distance > bm.max_move {
                        bm.set_pos2d(origin.lerp(bm.pos2d(), bm.max_move / distance));
                    }
                    if let Some(f) = fix_position.as_ref() {
                        let fixed = f(body, bm.position, body.position);
                        bm.fix_offset = fixed.truncate() - bm.position.truncate();
                        bm.position = fixed;
                    }
                } else {
                    bm.set_pos2d(origin);
                }
            }
        }

        for (body, bm) in bodies.iter_mut().zip(body_move.iter().skip(1)) {
            body.position = bm.position;
        }
    }

    pub fn get_path(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.pos_rec.iter().map(|rec| rec.value)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SnakeBody {
    pub delay: f32,
    pub distance: f32,
    pub position: Vec3,
    pub target: Vec3,
    pub segment: usize,
}

impl SnakeBody {
    pub fn new(delay: f32, distance: f32, position: Vec3) -> Self {
        Self {
            delay,
            distance,
            position,
            target: Vec3::ZERO,
            segment: 0,
        }
    }
}
