use bevy::math::*;

#[derive(Clone, Copy)]
struct MoveRecord {
    time: f32,
    distance: f32,
    position: Vec2,
}

pub struct SnakeHead {
    records: Vec<MoveRecord>,
}

impl SnakeHead {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }
    pub fn reset(&mut self, position: Vec2, time: f32) {
        self.records.clear();
        self.records.push(MoveRecord {
            time,
            distance: 0.0,
            position,
        });
        self.records.push(self.records[0]);
    }
    pub fn move_head(&mut self, position: Vec2, time: f32) {
        if self.records.len() < 2 {
            self.reset(position, time);
        }
        let last0 = self.records[self.records.len() - 1];
        let last1 = self.records[self.records.len() - 2];
        let dis1 = position.distance_squared(last1.position);
        if last0.distance - last1.distance < 1.0 && dis1 < 1.0 {
            self.records.pop();
            self.records.push(MoveRecord {
                time,
                distance: last1.distance + dis1.sqrt(),
                position,
            });
        } else {
            self.records.push(MoveRecord {
                time,
                distance: last0.distance + position.distance(last0.position),
                position,
            });
        }

        if self.records.len() > 2048 {
            self.records.drain(..128);
        }
    }
    pub fn get_record(&self, delay: f32, distance: f32) -> (Vec2, f32) {
        let last_rec = self.records.last().unwrap();
        let time = last_rec.time - delay;

        let time_index = match self
            .records
            .binary_search_by(|rec| rec.time.partial_cmp(&time).unwrap())
        {
            Ok(i) => i,
            Err(i) => i,
        }
        .min(self.records.len() - 1);
        if time_index == 0 {
            return (self.records[0].position, distance);
        }

        let rec0 = &self.records[time_index - 1];
        let rec1 = &self.records[time_index];
        let k = (time - rec0.time) / (rec1.time - rec0.time);
        let dis = k * (rec1.distance - rec0.distance) + rec0.distance - distance;
        let dis_index = match self
            .records
            .binary_search_by(|rec| rec.distance.partial_cmp(&dis).unwrap())
        {
            Ok(i) => i,
            Err(i) => i,
        }
        .min(self.records.len() - 1);
        if dis_index > 0 {
            let rec0 = &self.records[dis_index - 1];
            let rec1 = &self.records[dis_index];
            let k = (dis - rec0.distance) / (rec1.distance - rec0.distance);
            (rec0.position.lerp(rec1.position, k), 0.0)
        } else {
            (self.records[0].position, self.records[0].distance - dis)
        }
    }

    pub fn solve_body(&self, body: &mut [SnakeBody], dt: f32, speed: f32, radius: f32) {
        let max_move = dt * speed * 2.0;
        let head_pos = self.records.last().unwrap().position;

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
                let mut pos = t.clone();
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
    delay: f32,
    distance: f32,
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
