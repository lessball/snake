use bevy::math::*;

pub struct SnakeHead {
    position: Vec2,
    distance_record: Vec<(f32, f32)>,
    position_record: Vec<(f32, Vec2)>
}

impl SnakeHead {
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            distance_record: Vec::new(),
            position_record: Vec::new()
        }
    }

    pub fn reset(&mut self, position: Vec2, time: f32) {
        self.distance_record.clear();
        self.position_record.clear();
        self.distance_record.push((time, 0.0));
        self.position_record.push((0.0, position));
    }

    pub fn move_head(&mut self, position: Vec2, time: f32) {
        self.position = position;
        let n_dis = self.distance_record.len();
        let n_pos = self.position_record.len();
        if n_dis > 0 && n_pos > 0 {
            let last_dis = self.distance_record[n_dis - 1].1;
            let max_back = 40.0;
            let mut min_dis = f32::MAX;
            let mut index = usize::MAX;
            for i in (0..n_pos-1).rev() {
                let p = &self.position_record[i];
                if p.0 < last_dis - max_back {
                    break;
                }
                let dis = p.1.distance_squared(position);
                if dis < min_dis {
                    min_dis = dis;
                    index = i;
                }
            }
            if index < n_pos {
                let p = &self.position_record[index];
                if p.1.distance(position) + p.0 < last_dis {
                    self.position_record.truncate(index + 1);
                }
            }
            if let Some(p) = self.position_record.last() {
                let delta = p.1.distance(position);
                let new_dis = last_dis.max(p.0 + delta);
                if n_dis > 1 && new_dis - self.distance_record[n_dis - 2].1 < 0.0001 {
                    self.distance_record[n_dis - 1].0 = time;
                } else {
                    self.distance_record.push((time, new_dis));
                }
                let min_step = 5.0;
                if delta >= min_step {
                    let dis = p.0 + delta;
                    self.position_record.push((dis, position));
                }
            }
        } else {
            self.reset(position, time);
        }
    }

    pub fn get_record(&self, delay: f32, distance: f32) -> (Vec2, f32) {
        if let Some(d) = self.distance_record.last() {
            let time = d.0 - delay;
            let p_dis = match self.distance_record.binary_search_by(|(t, _)| t.partial_cmp(&time).unwrap()) {
                Ok(p) => p,
                Err(p) => p
            };
            if p_dis == 0 {
                return (self.position_record[0].1, distance);
            }
            let dis_rec0 = self.distance_record[p_dis - 1];
            let dis_rec1 = self.distance_record[p_dis];
            let k_dis = (time - dis_rec0.0) / (dis_rec1.0 - dis_rec0.0);
            let dis = k_dis * (dis_rec1.1 - dis_rec0.1) + dis_rec0.1 - distance;
            let p_pos = match self.position_record.binary_search_by(|(d, _)| d.partial_cmp(&dis).unwrap()) {
                Ok(p) => p,
                Err(p) => p
            };
            if p_pos > 0 {
                let pos_rec0 = &self.position_record[p_pos - 1];
                let pos_rec1 = &self.position_record[p_pos];
                let k_pos = (dis - pos_rec0.0) / (pos_rec1.0 - pos_rec0.0);
                (pos_rec0.1.lerp(pos_rec1.1, k_pos), 0.0)
            } else {
                (self.position_record[0].1, self.position_record[0].0 - dis)
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
