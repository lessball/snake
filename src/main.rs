use bevy::prelude::*;

const RADIUS: f32 = 30.0;
const RADIUS_HARD: f32 = 20.0;
const DISTANCE: f32 = 80.0;

#[derive(Clone, Copy)]
struct MoveRecord {
    time: f32,
    distance: f32,
    position: Vec2,
}

struct Leader {
    records: Vec<MoveRecord>,
    followers: Vec<Entity>,
}

impl Leader {
    pub fn new(followers: Vec<Entity>) -> Self {
        Leader {
            records: vec![MoveRecord {
                time: 0.0,
                distance: 0.0,
                position: Vec2::ZERO,
            }],
            followers,
        }
    }
    pub fn record(&mut self, position: Vec2, time: f32) {
        let last = self.records[self.records.len() - 1];
        let dis_sq = position.distance_squared(last.position);
        if dis_sq > 0.0001 {
            self.records.push(MoveRecord {
                time,
                distance: last.distance + dis_sq.sqrt(),
                position
            });
        } else if self.records.len() <= 1
            || last.distance - self.records[self.records.len() - 2].distance > 0.0001
        {
            self.records.push(MoveRecord {
                time,
                distance: last.distance,
                position
            });
        } else {
            let len = self.records.len();
            self.records[len - 1].time = time;
        }
        if self.records.len() > 2048 {
            self.records.drain(..128);
        }
    }
    pub fn get_record(&self, time: f32, distance: f32) -> Vec2 {
        let find0 = match self
            .records
            .binary_search_by(|rec| rec.time.partial_cmp(&time).unwrap())
        {
            Ok(i) => i,
            Err(i) => i,
        };
        let dis = if find0 > 0 && find0 < self.records.len() {
            let k = (time - self.records[find0 - 1].time)
                / (self.records[find0].time - self.records[find0 - 1].time);
            (1.0 - k) * self.records[find0 - 1].distance + k * self.records[find0].distance
                - distance
        } else{
            self.records[0].distance - distance
        };
        let find1 = match self
            .records
            .binary_search_by(|rec| rec.distance.partial_cmp(&dis).unwrap())
        {
            Ok(i) => i,
            Err(i) => i,
        };
        if find1 > 0 && find1 < self.records.len() {
            let k = (dis - self.records[find1 - 1].distance)
                / (self.records[find1].distance - self.records[find1 - 1].distance);
            self.records[find1 - 1]
                .position
                .lerp(self.records[find1].position, k)
        } else {
            self.records[0].position + Vec2::new(dis - self.records[0].distance, 0.0)
        }
    }
}

struct Follower {
    pub last_target: Vec2, //TODO
}

impl Follower {
    pub fn new(pos: Vec2) -> Self {
        Follower { last_target: pos }
    }
}

fn update(
    time: Res<Time>,
    windows: Res<Windows>,
    keyboard_input: Res<Input<KeyCode>>,
    mousebutton_input: Res<Input<MouseButton>>,
    query_leader: Query<(Entity, &mut Leader)>,
    mut query_trans: Query<&mut Transform>,
) {
    let mut dir = Vec2::ZERO;
    if keyboard_input.pressed(KeyCode::W) {
        dir.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::A) {
        dir.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) {
        dir.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::D) {
        dir.x += 1.0;
    }
    if dir.length_squared() > 0.1 {
        dir = dir.normalize();
    }
    let mut cursor_pos = None;
    if let Some(win) = windows.get_primary() {
        if let Some(p) = win.cursor_position() {
            cursor_pos = Some(p - Vec2::new(win.width() as f32, win.height() as f32) * 0.5);
        }
    }
    query_leader.for_each_mut(|(entity, mut leader)| {
        let mut leader_pos = query_trans.get_mut(entity).map_or(Vec2::ZERO, |tm| tm.translation.truncate());
        const SPEED: f32 = 120.0;
        let mut leader_dir = dir;
        if mousebutton_input.pressed(MouseButton::Left) {
            if let Some(p) = cursor_pos {
                let t = p - leader_pos;
                if t.length_squared() > 1.0 {
                    leader_dir = t.normalize();
                } else {
                    leader_dir = Vec2::ZERO;
                }
            }
        };
        let leader_delta = leader_dir * (time.delta_seconds() * SPEED);
        leader_pos += leader_delta;
        if let Ok(mut tm) = query_trans.get_mut(entity) {
            tm.translation = leader_pos.extend(0.0);
        }
        let now = time.seconds_since_startup() as f32;
        leader.record(leader_pos, now);

        let mut target: Vec<_> = (0..leader.followers.len())
            .map(|i| {
                let offset = (i + 1) as f32;
                leader.get_record(now - offset * 0.1, offset * DISTANCE)
            })
            .collect();
        for _ in 0..10 {
            for i in 0..target.len() {
                let v = target[i] - leader_pos;
                if v.length_squared() < RADIUS * RADIUS * 4.0 {
                    target[i] = leader_pos + v * (2.0 * RADIUS / v.length());
                }
            }
            for i in 0..target.len() - 1 {
                for j in i + 1..target.len() {
                    let v = target[j] - target[i];
                    if v.length_squared() < RADIUS * RADIUS * 4.0 {
                        let center = (target[i] + target[j]) * 0.5;
                        let r = v * (RADIUS / v.length());
                        target[i] = center - r;
                        target[j] = center + r;
                    }
                }
            }
        }
        let mut position: Vec<_> = leader
            .followers
            .iter()
            .enumerate()
            .map(|(i, e)| {
                if let Ok(tm) = query_trans.get_mut(*e) {
                    tm.translation.truncate()
                } else {
                    target[i]
                }
            })
            .collect();

        let move_step = time.delta_seconds() * SPEED * 1.25 / 5.0;
        let mut move_delta = vec![Vec2::ZERO; position.len()];
        for _ in 0..5 {
            for i in 0..position.len() {
                let v = target[i] - position[i];
                if v.length_squared() <= move_step * move_step {
                    move_delta[i] = v;
                    position[i] = target[i];
                } else {
                    move_delta[i] = v * (move_step / v.length());
                    position[i] += move_delta[i];
                }
            }
            for i in 0..position.len() {
                let v = position[i] - leader_pos;
                if v.length_squared() < RADIUS_HARD * RADIUS_HARD * 4.0 {
                    let len = v.length();
                    let rot = if leader_delta.dot(v) < 0.0 || move_delta[i].dot(v) > 0.0 {
                        0.0
                    } else if leader_delta.x * v.y - leader_delta.y * v.x
                        > move_delta[i].x * v.y - move_delta[i].y * v.x
                    {
                        1.0
                    } else {
                        -1.0
                    } * (2.0 - len / RADIUS_HARD);
                    let (sinr, cosr) = (rot + v.y.atan2(v.x)).sin_cos();
                    let r = Vec2::new(cosr, sinr) * (2.0 * RADIUS_HARD);
                    position[i] = leader_pos + r;
                }
            }
            for i in 0..position.len() - 1 {
                for j in i + 1..position.len() {
                    let v = position[j] - position[i];
                    if v.length_squared() < RADIUS_HARD * RADIUS_HARD * 4.0 {
                        let center = (position[i] + position[j]) * 0.5;
                        let len = v.length();
                        let rot = if move_delta[i].dot(v) < 0.0 || move_delta[j].dot(v) > 0.0 {
                            0.0
                        } else if move_delta[i].x * v.y - move_delta[i].y * v.x
                            > move_delta[j].x * v.y - move_delta[j].y * v.x
                        {
                            1.0
                        } else {
                            -1.0
                        } * (2.0 - len / RADIUS_HARD);
                        let (sinr, cosr) = (rot + v.y.atan2(v.x)).sin_cos();
                        let r = Vec2::new(cosr, sinr) * RADIUS_HARD;
                        position[i] = center - r;
                        position[j] = center + r;
                    }
                }
            }
        }

        for (i, e) in leader.followers.iter().enumerate() {
            if let Ok(mut tm) = query_trans.get_mut(*e) {
                tm.translation = position[i].extend(0.0);
            }
        }
    });
}

fn get_color(p: f32) -> Color {
    let key = [
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
    ];
    let t = p * (key.len() - 1) as f32;
    let index = t as usize;
    let k = t - index as f32;
    let mut rgb = [0.0; 3];
    for i in 0..3 {
        rgb[i] = key[index][i] * (1.0 - k) + key[index + 1][i] * k;
    }
    Color::rgb(rgb[0], rgb[1], rgb[2])
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    use bevy::render::mesh::{Indices, VertexAttributeValues};
    let mut mesh = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);
    let mut indices = Vec::new();
    let mut vertices = vec![[0.0f32; 3]];
    let normals = vec![[0.0, 0.0, 1.0]; 33];
    let uvs = vec![[0.0; 2]; 33];
    for i in 0..32 {
        indices.push(0);
        indices.push(i + 1);
        indices.push(((i + 1) % 32) + 1);
        let a = i as f32 * std::f32::consts::PI / 16.0;
        vertices.push([a.sin() * RADIUS, a.cos() * RADIUS, 0.0]);
    }
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.set_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float3(vertices),
    );
    mesh.set_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float3(normals),
    );
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float2(uvs));
    let circle = meshes.add(mesh);

    let followers: Vec<_> = (1..10)
        .map(|i| {
            let pos = Vec2::new(i as f32 * -DISTANCE, 0.0);
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite::new(Vec2::ONE),
                    material: materials.add(get_color(i as f32 / 10.0).into()),
                    mesh: circle.clone(),
                    transform: Transform::from_translation(pos.extend(0.0)),
                    ..Default::default()
                })
                .insert(Follower::new(pos))
                .id()
        })
        .collect();
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite::new(Vec2::ONE),
            material: materials.add(get_color(0.0).into()),
            mesh: circle,
            ..Default::default()
        })
        .insert(Leader::new(followers));
}

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(update.system())
        .run();
}
