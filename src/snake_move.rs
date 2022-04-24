use bevy::math::*;

struct DistanceRecord {
    time: f32,
    distance: f32
}

struct PositionRecord {
    distance: f32,
    position: Vec2
}

pub struct SnakeHead {
    position: Vec2,
    dis_rec: Vec<DistanceRecord>,
    pos_rec: Vec<PositionRecord>
}

impl SnakeHead {
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            dis_rec: Vec::new(),
            pos_rec: Vec::new()
        }
    }

    pub fn reset(&mut self, position: Vec2, time: f32) {
        self.position = position;
        self.dis_rec.clear();
        self.pos_rec.clear();
        self.dis_rec.push(DistanceRecord {
            time,
            distance: 0.0
        });
        self.pos_rec.push(PositionRecord{
            distance: 0.0,
            position
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
            for i in (0..n_pos-1).rev() {
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
                self.dis_rec.push(DistanceRecord{
                    time,
                    distance: new_dis
                });
                }
                let min_step = 5.0;
                if delta >= min_step {
                    let distance = last_pos.distance + delta;
                    self.pos_rec.push(PositionRecord {
                        distance,
                        position
                    });
                }
                if self.dis_rec.len() > 2048 {
                    self.dis_rec.drain(..128);
                }
                if self.pos_rec.len() >= 2048 {
                    self.pos_rec.drain(..128);
                }
            } else {
            self.reset(position, time);
        }
    }

    pub fn get_record(&self, delay: f32, distance: f32) -> (Vec2, f32) {
        if let Some(last_dis_rec) = self.dis_rec.last() {
            let time = last_dis_rec.time - delay;
            let p_dis = match self.dis_rec.binary_search_by(|d| d.time.partial_cmp(&time).unwrap()) {
                Ok(p) => p,
                Err(p) => p
            };
            if p_dis == 0 {
                return (self.pos_rec[0].position, distance);
            }
            let dis_rec0 = &self.dis_rec[p_dis - 1];
            let dis_rec1 = &self.dis_rec[p_dis];
            let k_dis = (time - dis_rec0.time) / (dis_rec1.time - dis_rec0.time);
            let dis = k_dis * (dis_rec1.distance - dis_rec0.distance) + dis_rec0.distance - distance;
            let p_pos = match self.pos_rec.binary_search_by(|p| p.distance.partial_cmp(&dis).unwrap()) {
                Ok(p) => p,
                Err(p) => p
            };
            if p_pos > 0 {
                let pos_rec0 = &self.pos_rec[p_pos - 1];
                let pos_rec1 = &self.pos_rec[p_pos];
                let k_pos = (dis - pos_rec0.distance) / (pos_rec1.distance - pos_rec0.distance);
                (pos_rec0.position.lerp(pos_rec1.position, k_pos), 0.0)
            } else {
                (self.pos_rec[0].position, self.pos_rec[0].distance - dis)
            }
        } else {
            (self.position, distance)
        }
    }

    pub fn solve_body(&self, body: &mut [SnakeBody], dt: f32, speed: f32, radius: f32) {
        let max_move = dt * speed * 2.0;
        let head_pos = self.position;

        let target: Vec<_> = body
            .iter()
            .map(|b| {
                let (p, d) = self.get_record(b.delay, b.distance);
                if d <= 0.0 {
                    p
                } else {
                    let v = b.position - p;
                    if v.length_squared() > d * d {
                        p + v * (d / v.length())
                    } else {
                        b.position
                    }
                }
            })
            .collect();
        let mut positions_new: Vec<_> = body
            .iter()
            .zip(target.iter())
            .map(|(b, t)| {
                let mut pos = *t;
                let v = pos - b.position;
                if v.length_squared() > max_move * max_move {
                    pos = b.position + v * (max_move / v.length());
                }
                pos
            })
            .collect();

        for _ in 0..4 {
            for i in 0..body.len() {
                let v = positions_new[i] - head_pos;
                if v.length_squared() < radius * radius * 4.0 {
                    let len = v.length();
                    let pop_dis = (radius * 2.0 - len).min(max_move);
                    if v.dot(target[i] - head_pos) < -0.001 {
                        let mut angle = pop_dis * 0.5 / radius;
                        let dir = target[i] - positions_new[i];
                        let cross = dir.x * v.y - dir.y * v.x;
                        if 0.0 < cross {
                            angle = -angle;
                        }
                        let offset = Mat2::from_angle(angle).mul_vec2(v) * (1.0 + pop_dis / len);
                        positions_new[i] = head_pos + offset;
                    } else {
                        positions_new[i] += v * (pop_dis / len);
                    }
                }
            }
            for i in 0..body.len() - 1 {
                for j in i + 1..body.len() {
                    let v = positions_new[j] - positions_new[i];
                    if v.length_squared() < radius * radius * 4.0 {
                        let len = v.length();
                        let pop_dis = (radius - len * 0.5).min(max_move);
                        if v.dot(target[j] - target[i]) < -0.001 {
                            let mut angle = pop_dis / radius;
                            let dir0 = target[i] - positions_new[i];
                            let dir1 = target[j] - positions_new[j];
                            let cross0 = dir0.x * v.y - dir0.y * v.x;
                            let cross1 = dir1.x * v.y - dir1.y * v.x;
                            if cross0 < cross1 {
                                angle = -angle;
                            }
                            let offset =
                                Mat2::from_angle(angle).mul_vec2(v) * (0.5 + pop_dis / len);
                            let center = (positions_new[i] + positions_new[j]) * 0.5;
                            positions_new[i] = center - offset;
                            positions_new[j] = center + offset;
                        } else {
                            let pop = v * (pop_dis / len);
                            positions_new[i] -= pop;
                            positions_new[j] += pop;
                        }
                    }
                }
            }
        }
        for i in 0..body.len() {
            body[i].position = positions_new[i];
        }
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
