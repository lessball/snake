use glam::{Mat2, Vec2, Vec3, Vec3Swizzles};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const SOLVE_STEP: i32 = 8;

fn invert_lerp<T: num_traits::Float>(min: T, max: T, k: T) -> T {
    (k - min) / (max - min)
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
struct MoveRecord {
    time: f64,
    distance: f64,
    position: Vec3,
}

impl MoveRecord {
    fn pos2d(&self) -> Vec2 {
        self.position.xy()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MoveMode {
    Normal,
    Teleport,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct ModeRecord {
    distance: f64,
    mode: MoveMode,
    position: Vec3,
}

#[cfg_attr(feature = "serde", derive(Clone, Serialize, Deserialize))]
pub struct SnakeHead {
    time: f64,
    max_distance: f64,
    move_rec: Vec<MoveRecord>,
    mode_rec: Vec<ModeRecord>,
    pub bodies: Vec<SnakeBody>,
}

impl SnakeHead {
    pub fn new(bodies: Vec<SnakeBody>) -> Self {
        assert!(!bodies.is_empty(), "snake bodies is empty");
        Self {
            time: 0.0,
            max_distance: 0.0,
            move_rec: Vec::new(),
            mode_rec: Vec::new(),
            bodies,
        }
    }

    pub fn head_position(&self) -> Vec3 {
        self.bodies[0].position
    }

    pub fn trim_head(&mut self, index: usize) {
        let distance = self.bodies[index].move_distance;
        self.max_distance -= self.move_rec.last().unwrap().distance - distance;
        let p = self
            .move_rec
            .partition_point(|rec| rec.distance <= distance);
        if p > 0 && p < self.move_rec.len() {
            self.move_rec.truncate(p + 1);
            let a = &self.move_rec[p - 1];
            let b = &self.move_rec[p];
            let k = invert_lerp(a.distance, b.distance, distance);
            self.time = a.time + (b.time - a.time) * k;
            let position = a.position.lerp(b.position, k as f32);
            self.move_rec[p] = MoveRecord {
                time: self.time,
                distance,
                position,
            };
        } else if p == 0 {
            self.max_distance = 0.0;
            let position = self.bodies[index].position;
            self.move_rec.clear();
            self.move_rec.push(MoveRecord {
                time: self.time,
                distance,
                position,
            });
        }
        self.mode_rec.truncate(self.bodies[index].segment + 1);
        if self.mode_rec.is_empty() {
            self.mode_rec.push(ModeRecord {
                distance,
                mode: MoveMode::Normal,
                position: self.bodies.last().unwrap().position,
            });
        }
    }

    pub fn move_head(&mut self, dt: f64, position: Vec3, move_mode: MoveMode) {
        self.time += dt;
        if !self.move_rec.is_empty() {
            let pos2d = position.xy();
            let new_seg = match move_mode {
                MoveMode::Teleport => true,
                _ => move_mode != self.mode_rec.last().unwrap().mode,
            };
            if new_seg {
                let pos1 = match move_mode {
                    MoveMode::Teleport => position,
                    _ => self.move_rec.last().unwrap().position,
                };
                self.mode_rec.push(ModeRecord {
                    distance: self.move_rec.last().unwrap().distance,
                    mode: move_mode,
                    position: pos1,
                });
            }

            // move back, remove record
            let back_limit = self.max_distance - 40.0;
            let mut min_dis = f32::MAX;
            let mut index = usize::MAX;
            for i in (0..self.move_rec.len() - 1).rev() {
                let p = &self.move_rec[i];
                if p.distance <= back_limit {
                    break;
                }
                let dis = p.pos2d().distance_squared(pos2d);
                if dis < min_dis {
                    min_dis = dis;
                    index = i;
                }
            }
            if index < self.move_rec.len() {
                let p = &self.move_rec[index];
                if p.pos2d().distance(pos2d) as f64 + p.distance < self.max_distance {
                    self.move_rec.truncate(index + 1);
                }
            }

            // add record
            let last_rec = self.move_rec.last().unwrap();
            let cur_dis = match move_mode {
                MoveMode::Teleport => last_rec.distance,
                _ => last_rec.distance + last_rec.pos2d().distance(pos2d) as f64,
            };
            self.max_distance = self.max_distance.max(cur_dis);
            if !new_seg
                && self.move_rec.len() > 1
                && cur_dis - self.move_rec[self.move_rec.len() - 2].distance < 0.0001
            {
                // no move, replace last record
                self.move_rec.pop();
            }
            self.move_rec.push(MoveRecord {
                time: self.time,
                distance: cur_dis,
                position,
            });
        } else {
            self.move_rec.push(MoveRecord {
                time: self.time,
                distance: 0.0,
                position,
            });
            self.mode_rec.push(ModeRecord {
                distance: 0.0,
                mode: move_mode,
                position,
            });
        }
        let body0 = &mut self.bodies[0];
        body0.position = position;
        body0.target = position;
        body0.move_distance = self.move_rec.last().unwrap().distance;
        body0.segment = self.mode_rec.len() - 1;
    }

    pub fn update_body(&mut self, radius: f32) {
        if self.move_rec.is_empty() {
            return;
        }
        let head_pos = self.head_position().xy();
        let rr4 = radius * radius * 4.0;
        let mut min_distance = f64::MAX;
        let mut min_segment = usize::MAX;

        let bodies = &mut self.bodies;
        for i in 1..bodies.len() {
            let (bodies0, bodies1) = bodies.split_at_mut(i);
            let (body, _) = bodies1.split_first_mut().unwrap();
            let time = self.time - body.delay as f64;
            let p = self.move_rec.partition_point(|rec| rec.time < time);
            let mut distance = if p > 0 && p < self.move_rec.len() {
                let a = &self.move_rec[p - 1];
                let b = &self.move_rec[p];
                a.distance + (b.distance - a.distance) * invert_lerp(a.time, b.time, time)
            } else if p == 0 {
                self.move_rec[0].distance
            } else {
                self.move_rec.last().unwrap().distance
            };
            distance = body.move_distance.max(distance - body.distance as f64);
            body.move_distance = distance;
            min_distance = min_distance.min(distance);
            let iseg = body.segment;
            if iseg + 1 < self.mode_rec.len() {
                let can_leave = match self.mode_rec[iseg].mode {
                    MoveMode::Normal => distance >= self.mode_rec[iseg + 1].distance,
                    MoveMode::Teleport => true,
                };
                if can_leave {
                    match self.mode_rec[iseg + 1].mode {
                        MoveMode::Normal => {
                            body.segment += 1;
                        }
                        MoveMode::Teleport => {
                            let pos = self.mode_rec[iseg + 1].position;
                            let pos2d = pos.xy();
                            if pos2d.distance_squared(head_pos) >= rr4
                                && bodies0
                                    .iter()
                                    .all(|body| pos2d.distance_squared(body.position.xy()) >= rr4)
                            {
                                body.position = pos;
                                body.segment += 1;
                            }
                        }
                    };
                }
                min_segment = min_segment.min(body.segment);
            }
            match self.mode_rec[iseg].mode {
                MoveMode::Normal => {
                    body.target = if distance > self.mode_rec[iseg].distance {
                        let p = self.move_rec.partition_point(|rec| rec.distance < distance);
                        if p > 0 && p < self.move_rec.len() {
                            let a = &self.move_rec[p - 1];
                            let b = &self.move_rec[p];
                            a.position.lerp(
                                b.position,
                                invert_lerp(a.distance, b.distance, distance) as f32,
                            )
                        } else if p == 0 {
                            self.move_rec[0].position
                        } else {
                            self.move_rec.last().unwrap().position
                        }
                    } else {
                        let p0 = self.mode_rec[iseg].position;
                        let remain = (self.mode_rec[iseg].distance - distance) as f32;
                        let dis = p0.xy().distance(body.position.xy());
                        if dis > remain {
                            p0.lerp(body.position, remain / dis)
                        } else {
                            body.position
                        }
                    };
                }
                MoveMode::Teleport => {
                    body.target = body.position;
                }
            }
        }
        if min_segment > 0 && min_segment < self.mode_rec.len() {
            self.mode_rec.drain(0..min_segment);
            for body in bodies.iter_mut().skip(1) {
                body.segment -= min_segment;
            }
        }
        let rec_len = self.move_rec.len();
        if rec_len > 2 && self.move_rec[rec_len / 2 + 1].distance < min_distance {
            self.move_rec.drain(..rec_len / 2);
        }
    }

    fn foreach_pair<F: FnMut(usize, usize)>(len: usize, mut f: F) {
        for i in 0..len - 1 {
            for j in i + 1..len {
                f(i, j);
            }
        }
    }

    pub fn solve_body<F>(
        &mut self,
        max_move: f32,
        min_move: f32,
        radius: f32,
        fix_position: Option<F>,
    ) where
        F: Fn(&SnakeBody, Vec3, Vec3) -> Vec3,
    {
        let rr4 = radius * radius * 4.0;

        let bodies = &mut self.bodies;
        bodies[0].max_move = 0.0;
        bodies[0].delta = Vec2::ZERO;
        bodies[0].position_prev = bodies[0].position;
        for body in bodies.iter_mut().skip(1) {
            body.max_move = max_move;
            body.delta = Vec2::ZERO;
            body.position_prev = body.position;
            let target_distance = body.target.xy().distance(body.position.xy());
            if target_distance > 0.0001 {
                let k: f32 = invert_lerp(1.5, 4.0, target_distance / radius).clamp(0.0, 1.0);
                body.max_move = max_move * (1.5 + k * 0.5);
                body.delta = body.target.xy() - body.position.xy();
                if target_distance > body.max_move {
                    body.delta *= body.max_move / target_distance;
                }
            }
        }

        for _ in 0..SOLVE_STEP {
            for body in bodies.iter_mut().skip(1) {
                body.add_pos2d(body.delta / SOLVE_STEP as f32);
            }
            Self::foreach_pair(bodies.len(), |i, j| {
                let body0 = &bodies[i];
                let body1 = &bodies[j];
                if !(body0.collision && body1.collision) {
                    return;
                }
                if (body0.position.z - body1.position.z).abs() > radius * 2.0 {
                    return;
                }
                if body0.pos2d().distance_squared(body1.pos2d()) >= rr4 {
                    return;
                }
                let v0 = body1.pos2d() - body0.pos2d();
                let len = v0.length();
                let d = if len > 0.0001 {
                    v0 * (radius / len - 0.5)
                } else {
                    let (x, y) = ((i * bodies.len() + j) as f32).sin_cos();
                    radius * Vec2::new(x, y)
                };
                if body0.max_move > 0.0 {
                    bodies[i].add_pos2d(-d);
                    bodies[j].add_pos2d(d);
                } else {
                    bodies[j].add_pos2d(d * 2.0);
                }
            });
            Self::foreach_pair(bodies.len(), |i, j| {
                let body0 = &bodies[i];
                let body1 = &bodies[j];
                if !(body0.collision && body1.collision) {
                    return;
                }
                if (body0.position.z - body1.position.z).abs() > radius * 2.0 {
                    return;
                }
                if body0.pos2d().distance_squared(body1.pos2d()) >= rr4 * 1.0001 {
                    return;
                }
                let dp = body1.pos2d() - body0.pos2d();
                if dp.dot(body1.target.xy() - body0.target.xy()) >= -0.001 {
                    return;
                }
                let vertical = Vec2::new(dp.y, -dp.x);
                let mut angle = 24.0 * max_move / radius / SOLVE_STEP as f32;
                let rand_offset = (body0.position + body1.position)
                    .as_ref()
                    .iter()
                    .sum::<f32>()
                    .sin_cos();
                if (body0.delta - body1.delta + Vec2::new(rand_offset.0, rand_offset.1))
                    .dot(vertical)
                    < 0.0
                {
                    angle = -angle;
                }
                let offset = Mat2::from_angle(angle).mul_vec2(dp) - dp;
                // if body0.fix_offset.dot(offset) > 0.0 || body1.fix_offset.dot(offset) < 0.0 {
                //     return;
                // }
                let check_move = |pos: Vec2| {
                    for (k, body) in bodies.iter().enumerate() {
                        if k != i && k != j && pos.distance_squared(body.pos2d()) < rr4 {
                            return false;
                        }
                    }
                    true
                };
                if body0.max_move > 0.0 {
                    let pos0 = body0.pos2d() - offset;
                    let pos1 = body1.pos2d() + offset;
                    if check_move(pos0) && check_move(pos1) {
                        bodies[i].set_pos2d(pos0);
                        bodies[j].set_pos2d(pos1);
                    }
                } else {
                    let pos = body1.pos2d() + 2.0 * offset;
                    if check_move(pos) {
                        bodies[j].set_pos2d(pos);
                    }
                }
            });
            for body in bodies.iter_mut().skip(1) {
                let origin = body.position_prev.xy();
                let distance = origin.distance(body.pos2d());
                // body.fix_offset = Vec2::ZERO;
                if distance >= min_move / SOLVE_STEP as f32 {
                    if distance > body.max_move {
                        body.set_pos2d(origin.lerp(body.pos2d(), body.max_move / distance));
                    }
                    if let Some(f) = fix_position.as_ref() {
                        let fixed = f(body, body.position, body.position_prev);
                        // body.fix_offset = fixed.truncate() - body.position.truncate();
                        body.position = fixed;
                    }
                } else {
                    body.set_pos2d(origin);
                }
            }
        }
    }

    pub fn get_path(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.move_rec.iter().map(|rec| rec.position)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SnakeBody {
    pub delay: f32,
    pub distance: f32,
    pub position: Vec3,
    pub target: Vec3,
    pub collision: bool,
    segment: usize,
    move_distance: f64,
    delta: Vec2,
    max_move: f32,
    position_prev: Vec3,
}

impl SnakeBody {
    pub fn new(delay: f32, distance: f32, position: Vec3) -> Self {
        Self {
            delay,
            distance,
            position,
            target: Vec3::ZERO,
            collision: true,
            segment: 0,
            move_distance: f64::MIN,
            delta: Vec2::ZERO,
            max_move: 0.0,
            position_prev: Vec3::ZERO,
        }
    }
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
