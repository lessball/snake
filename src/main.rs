use bevy::prelude::*;

mod snake_move;
use snake_move::*;

const RADIUS: f32 = 30.0;
const RADIUS_HARD: f32 = 25.0;
const DISTANCE: f32 = 80.0;

struct Leader {
    snake: SnakeMove,
    followers: Vec<Entity>,
}

impl Leader {
    pub fn new(followers: Vec<Entity>) -> Self {
        let mut snake = SnakeMove::new();
        snake.reset(Vec2::ZERO, Vec2::X, 0.0);
        Leader {
            snake,
            followers,
        }
    }
}

struct Follower {}

impl Follower {
    pub fn new() -> Self {
        Follower {}
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
    let mut input_dir = Vec2::ZERO;
    if keyboard_input.pressed(KeyCode::W) {
        input_dir.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::A) {
        input_dir.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) {
        input_dir.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::D) {
        input_dir.x += 1.0;
    }
    input_dir = input_dir.normalize_or_zero();
    let mut cursor_pos = None;
    if let Some(win) = windows.get_primary() {
        if let Some(p) = win.cursor_position() {
            cursor_pos = Some(p - Vec2::new(win.width() as f32, win.height() as f32) * 0.5);
        }
    }
    query_leader.for_each_mut(|(entity, mut leader)| {
        let mut leader_pos = query_trans
            .get_mut(entity)
            .map_or(Vec2::ZERO, |tm| tm.translation.truncate());
        const SPEED: f32 = 300.0;
        let mut leader_dir = input_dir;
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
        let delta_time = time.delta_seconds();
        let leader_delta = leader_dir * (delta_time * SPEED);
        leader_pos += leader_delta;
        if let Ok(mut tm) = query_trans.get_mut(entity) {
            tm.translation = leader_pos.extend(0.0);
        }
        let now = time.seconds_since_startup() as f32;
        leader.snake.record(leader_pos, now);

        let mut positions: Vec<_> = leader
            .followers
            .iter()
            .map(|e| {
                query_trans.get_mut(*e).map_or(Vec2::ZERO, |tm| tm.translation.truncate())
            })
            .collect();
        let step_count = (delta_time * 600.0).floor().max(1.0).min(5.0);
        let step_time = delta_time / step_count;
        for _ in 0..step_count as i32 {
            leader.snake.solve_followers(&mut positions,
                |i| ((i + 1) as f32 * 0.1, (i + 1) as f32 * DISTANCE),
                RADIUS,
                RADIUS_HARD,
                step_time * SPEED * 2.0
            );
        }
        for (i, e) in leader.followers.iter().enumerate() {
            if let Ok(mut tm) = query_trans.get_mut(*e) {
                tm.translation = positions[i].extend(0.0);
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
                .insert(Follower::new())
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
