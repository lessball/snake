use bevy::prelude::*;

#[derive(Clone, Copy)]
struct MoveRecord {
    time: f32,
    distance: f32,
    position: Vec2,
}

pub struct SnakeMove {
    records: Vec<MoveRecord>,
}

impl SnakeMove {
    pub fn new() -> Self {
        Self {
            records: Vec::new()
        }
    }
    pub fn reset(&mut self, position: Vec2, dir: Vec2, time: f32) {
        self.records.clear();
        self.records.push(MoveRecord {
            time: -1e10,
            distance: -10000.0,
            position: position - dir.normalize_or_zero() * 10000.0,
        });
        self.records.push(MoveRecord {
            time: -1e10,
            distance: 0.0,
            position,
        });
        self.records.push(MoveRecord {
            time,
            distance: 0.0,
            position,
        });
    }
    pub fn record(&mut self, position: Vec2, time: f32) {
        if self.records.len() < 3 {
            self.reset(position, Vec2::new(1.0, 0.0), time);
        }
        let last0 = self.records[self.records.len()-1];
        let last1 = self.records[self.records.len()-2];
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
    pub fn get_record(&self, delay: f32, distance: f32) -> Vec2 {
        let time = self.records.last().unwrap().time - delay;
        let find_time = match self
            .records
            .binary_search_by(|rec| rec.time.partial_cmp(&time).unwrap())
        {
            Ok(i) => i,
            Err(i) => i,
        };
        if find_time == 0 {
            return self.records[0].position;
        }
        let dis = if find_time < self.records.len() {
            let record_prev = &self.records[find_time - 1];
            let record_next = &self.records[find_time];
            let k = (time - record_prev.time) / (record_next.time - record_prev.time);
            k * (record_next.distance - record_prev.distance) + record_prev.distance - distance
        } else {
            self.records.last().unwrap().distance - distance
        };
        let find_dis = match self
            .records
            .binary_search_by(|rec| rec.distance.partial_cmp(&dis).unwrap())
        {
            Ok(i) => i,
            Err(i) => i,
        };
        if find_dis == 0 {
            return self.records[0].position;
        }
        if find_dis < self.records.len() {
            let record_prev = &self.records[find_dis - 1];
            let record_next = &self.records[find_dis];
            let k = (dis - record_prev.distance) / (record_next.distance - record_prev.distance);
            record_prev.position.lerp(record_next.position, k)
        } else {
            self.records.last().unwrap().position
        }
    }
    pub fn solve_followers<F>(
        &self,
        positions: &mut [Vec2],
        delay_and_distance: F,
        radius: f32,
        hard_radius: f32,
        max_move: f32)
        where F: Fn(usize) -> (f32,f32) {
        let leader_pos = self.records.last().unwrap().position;
        let leader_dir = (leader_pos - self.get_record(0.0, 4.0)).normalize_or_zero();
        
        let mut target: Vec<_> = (0..positions.len())
            .map(|i| {
                let (delay, distance) = delay_and_distance(i);
                self.get_record(delay, distance)
            })
            .collect();
        let dir: Vec<_> = (0..positions.len())
            .map(|i| {
                let (delay, distance) = delay_and_distance(i);
                (target[i] - self.get_record(delay, distance + 4.0)).normalize_or_zero()
            })
            .collect();

        let mut hit = vec![false; target.len()];

        for i in 0..target.len() {
            let v = target[i] - leader_pos;
            if v.length_squared() < radius * radius * 4.0 {
                let dir0 = leader_dir;
                let dir1 = dir[i];
                if dir0.dot(v) > 0.0 && dir1.dot(v) < 0.0 {
                    hit[i] = true;
                    let len = v.length();
                    let cross0 = dir0.x * v.y - dir0.y * v.x;
                    let cross1 = dir1.x * v.y - dir1.y * v.x;
                    let slide_len = (radius * radius * 4.0 - len * len).sqrt() + 0.1;
                    let mut slide = Vec2::new(-v.y, v.x) * (slide_len / len);
                    if cross0 > cross1 {
                        slide *= -1.0;
                    }
                    target[i] -= slide;
                }
            }
        }
        for i in 0..target.len() - 1 {
            if hit[i] {
                continue;
            }
            for j in i + 1..target.len() {
                if hit[j] {
                    continue;
                }
                let v = target[j] - target[i];
                if v.length_squared() < radius * radius * 4.0 {
                    let dir0 = dir[i];
                    let dir1 = dir[j];
                    let len = v.length();
                    if dir0.dot(v) > 0.0 && dir1.dot(v) < 0.0 {
                        hit[i] = true;
                        hit[j] = true;
                        let cross0 = dir0.x * v.y - dir0.y * v.x;
                        let cross1 = dir1.x * v.y - dir1.y * v.x;
                        let slide_len = (radius * radius * 4.0 - len * len).sqrt() * 0.5 + 0.1;
                        let mut slide = Vec2::new(-v.y, v.x) * (slide_len / len);
                        if cross0 > cross1 {
                            slide *= -1.0;
                        }
                        target[i] += slide;
                        target[j] -= slide;
                        break;
                    }
                }
            }
        }
        
        for i in 0..positions.len() {
            if target[i].distance_squared(leader_pos) < hard_radius * hard_radius * 0.25 {
                continue;
            }
            let delta = target[i] - positions[i];
            let delta_len_sq = delta.length_squared();
            if delta_len_sq <= max_move * max_move {
                positions[i] = target[i];
            } else {
                positions[i] += delta * (max_move / delta_len_sq.sqrt());
            }

            let to_leader = positions[i] - leader_pos;
            let len_sq = to_leader.length_squared();
            if len_sq < hard_radius * hard_radius * 4.0 {
                let len = len_sq.sqrt();
                let d = (hard_radius - len * 0.5).min(max_move);
                positions[i] += to_leader * (d / len);
            }
        }
        for i in 0..positions.len() - 1 {
            for j in i + 1..positions.len() {
                let v = positions[j] - positions[i];
                let len_sq = v.length_squared();
                if len_sq < hard_radius * hard_radius * 4.0 && len_sq > 0.001{
                    let len = len_sq.sqrt();
                    let d = (hard_radius - len * 0.5).min(max_move);
                    let pop = v * (d / len);
                    positions[j] += pop;
                    positions[i] -= pop;
                }
            }
        }
    }
}
