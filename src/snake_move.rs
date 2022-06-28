use bevy::math::*;

struct DistanceRecord {
    time: f32,
    distance: f32,
}

struct PositionRecord {
    distance: f32,
    position: Vec2,
}

pub struct SnakeHead {
    position: Vec2,
    dis_rec: Vec<DistanceRecord>,
    pos_rec: Vec<PositionRecord>,
    max_delay: f32,
    max_distance: f32
}

impl SnakeHead {
    pub fn new(max_delay: f32, max_distance: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            dis_rec: Vec::new(),
            pos_rec: Vec::new(),
            max_delay,
            max_distance
        }
    }

    pub fn reset(&mut self, position: Vec2, time: f32) {
        self.position = position;
        self.dis_rec.clear();
        self.pos_rec.clear();
        self.dis_rec.push(DistanceRecord {
            time,
            distance: 0.0,
        });
        self.pos_rec.push(PositionRecord {
            distance: 0.0,
            position,
        });
    }

    pub fn move_head(&mut self, position: Vec2, time: f32) {
        self.position = position;
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
                if p.position.distance(position) + p.distance < last_dis {
                    self.pos_rec.truncate(index + 1);
                }
            }

            // add record
            let last_pos = self.pos_rec.last().unwrap();
            let delta = last_pos.position.distance(position);
            let new_dis = last_dis.max(last_pos.distance + delta);
            if n_dis > 1 && new_dis - self.dis_rec[n_dis - 2].distance < 0.0001 {
                // merge same distance record
                self.dis_rec[n_dis - 1].time = time;
            } else {
                self.dis_rec.push(DistanceRecord {
                    time,
                    distance: new_dis,
                });
            }
            let min_step = 5.0;
            if delta >= min_step {
                let distance = last_pos.distance + delta;
                self.pos_rec.push(PositionRecord { distance, position });
            }
            if self.dis_rec.len() > 128 && self.dis_rec[128].time + self.max_delay < self.dis_rec.last().unwrap().time {
                self.dis_rec.drain(..128);
            }
            if self.pos_rec.len() > 128 && self.pos_rec[128].distance + self.max_distance < self.pos_rec.last().unwrap().distance {
                self.pos_rec.drain(..128);
            }
        } else {
            self.reset(position, time);
        }
    }

    pub fn get_distance(&self, time: f32) -> f32 {
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

    pub fn get_position(&self, distance: f32) -> (Vec2, f32) {
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
            (pos_rec0.position.lerp(pos_rec1.position, k), 0.0)
        } else if !self.pos_rec.is_empty() {
            let p1 = p.min(self.pos_rec.len() - 1);
            (
                self.pos_rec[p1].position,
                self.pos_rec[p1].distance - distance,
            )
        } else {
            (Vec2::ZERO, 0.0)
        }
    }

    pub fn solve_body(
        &self,
        body: &mut [SnakeBody],
        dt: f32,
        speed: f32,
        radius: f32,
    ) -> Vec<Vec2> {
        let max_move = dt * speed * 2.0;
        let head_pos = self.position;
        
        let head_moving = if self.dis_rec.len() > 1 {
            let last = &self.dis_rec[self.dis_rec.len() - 1];
            let prev = &self.dis_rec[self.dis_rec.len() - 2];
            last.distance > prev.distance + 0.001 || last.time < prev.time + 0.1
        } else {
            false
        };

        struct BodyMove {
            position: Vec2,
            target: Vec2,
            target_moving: bool,
            self_moving: bool,
            overlap: [f32; 2],
        }
        let mut body_move: Vec<_> = body
            .iter()
            .map(|body| {
                let time = self.dis_rec.last().map_or(0.0, |rec| rec.time - body.delay);
                let distance = self.get_distance(time);
                let (mut target, d) = self.get_position(distance - body.distance);
                if d > 0.0 {
                    let v = body.position - target;
                    if v.length_squared() > d * d {
                        target += v * (d / v.length());
                    } else {
                        target = body.position;
                    }
                }
                let target_moving = distance - self.get_distance(time - 0.1) > 0.0001;
                BodyMove {
                    position: body.position,
                    target,
                    target_moving,
                    self_moving: target_moving,
                    overlap: [0.0, 0.0]
                }
            }).collect();
        
        let calc_overlap = |body_move: &mut[BodyMove], target: usize| {
            for i in 0..body_move.len() {
                let d1 = body_move[i].position.distance_squared(head_pos);
                if d1 < radius * radius * 4.0 {
                    body_move[i].overlap[target] +=  radius * 2.0 - d1.sqrt();
                }
                for j in i + 1..body_move.len() {
                    let d2 = body_move[i].position.distance_squared(body_move[j].position);
                    if d2 < radius * radius * 4.0 {
                        let d = radius * 2.0 - d2.sqrt();
                        body_move[i].overlap[target] += d;
                        body_move[j].overlap[target] += d;
                    }
                }
            }
        };
        calc_overlap(&mut body_move, 0);
        for body in body_move.iter_mut() {
            let v = body.target - body.position;
            if v.length_squared() > max_move * max_move {
                body.position += v * (max_move / v.length());
            } else {
                body.position = body.target;
            }
        }

        let max_pop = max_move * 0.5;
        for _ in 0..4 {
            for i in 0..body_move.len() {
                let v = body_move[i].position - head_pos;
                if v.length_squared() < radius * radius * 4.0 {
                    let len = v.length();
                    let pop_dis = (radius * 2.0 - len).min(max_pop * 1.5);
                    if v.dot(body_move[i].target - head_pos) < -0.001 {
                        let mut angle = pop_dis * 0.125 / radius;
                        let dir = body_move[i].target - body_move[i].position;
                        let cross = dir.x * v.y - dir.y * v.x;
                        if 0.0 < cross {
                            angle = -angle;
                        }
                        let offset = Mat2::from_angle(angle).mul_vec2(v) * (1.0 + pop_dis / len);
                        body_move[i].position = head_pos + offset;
                    } else {
                        body_move[i].position += v * (pop_dis / len);
                    }
                    if head_moving {
                        body_move[i].self_moving = true;
                    }
                }
                for j in i + 1..body_move.len() {
                    let v = body_move[j].position - body_move[i].position;
                    if v.length_squared() < radius * radius * 4.0 {
                        let len = v.length();
                        let pop_dis = (radius - len * 0.5).min(max_pop);
                        if v.dot(body_move[j].target - body_move[i].target) < -0.001 {
                            let mut angle = pop_dis / radius * 0.25;
                            let dir0 = body_move[i].target - body_move[i].position;
                            let dir1 = body_move[j].target - body_move[j].position;
                            let cross0 = dir0.x * v.y - dir0.y * v.x;
                            let cross1 = dir1.x * v.y - dir1.y * v.x;
                            if cross0 < cross1 {
                                angle = -angle;
                            }
                            let offset =
                                Mat2::from_angle(angle).mul_vec2(v) * (0.5 + pop_dis / len);
                            let center = (body_move[i].position + body_move[j].position) * 0.5;
                            body_move[i].position = center - offset;
                            body_move[j].position = center + offset;
                        } else {
                            let pop = v * (pop_dis / len);
                            body_move[i].position -= pop;
                            body_move[j].position += pop;
                        }
                        if body_move[i].target_moving {
                            body_move[j].self_moving = true;
                        }
                        if body_move[j].target_moving {
                            body_move[i].self_moving = true;
                        }
                    }
                }
            }
        }
        calc_overlap(&mut body_move, 1);
        for (body, body_move) in body.iter_mut().zip(body_move.iter()) {
            if body_move.self_moving || body_move.overlap[1] < body_move.overlap[0] {
                body.position = body_move.position;
            }
        }
        body_move.iter().map(|body| body.target).collect()
    }

    pub fn get_path(&self) -> Vec<Vec2> {
        self.pos_rec.iter().map(|r| r.position).collect()
    }
}

#[derive(Clone, Default)]
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
