use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::pbr::NotShadowCaster;

use serde::{Deserialize, Serialize};

use snake_move::*;

mod line_poly;
use line_poly::LinePoly;

// mod test_plugin;
// use test_plugin::TestPlugin;

fn to_snake(v: Vec3) -> Vec3 {
    Vec3::new(v.x, -v.z, v.y)
}

fn from_snake(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.z, -v.y)
}

const RADIUS: f32 = 30.0;
const DISTANCE: f32 = 80.0;
const SPEED: f32 = 300.0;

#[derive(Component)]
struct Portal(Vec3);

#[derive(Component)]
struct Leader {
    snake_head: SnakeHead,
    snake_bodys: Vec<SnakeBody>,
    followers: Vec<Entity>,
    targets: Vec<Entity>,
    path_mesh: Handle<Mesh>,
}

fn leader_move(
    time: Res<Time>,
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    keyboard_input: Res<Input<KeyCode>>,
    mousebutton_input: Res<Input<MouseButton>>,
    mut query_leader: Query<(&mut Leader, &mut Transform)>,
    portal: Query<(&Portal, &Transform), (With<Portal>, Without<Leader>)>,
) {
    let input_dir = Vec3::new(
        keyboard_input.pressed(KeyCode::D) as i32 as f32 - keyboard_input.pressed(KeyCode::A) as i32 as f32,
        0.0,
        keyboard_input.pressed(KeyCode::S) as i32 as f32 - keyboard_input.pressed(KeyCode::W) as i32 as f32,
    )
    .normalize_or_zero();
    let cursor_pos = if mousebutton_input.pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera.single();
        let wnd = window.single();
        wnd.cursor_position().and_then(|pos| {
            camera
                .viewport_to_world(camera_transform, pos)
                .map(|ray| ray.origin - ray.direction * (ray.origin.y / ray.direction.y))
        })
    } else {
        None
    };
    for (mut leader, mut tm) in query_leader.iter_mut() {
        let mut leader_pos = tm.translation;
        let delta_time = time.delta_seconds();
        let mut teleport = false;
        for (pt, tm) in portal.iter() {
            if tm.translation.distance_squared(leader_pos) < RADIUS * RADIUS {
                leader_pos = pt.0;
                teleport = true;
                break;
            }
        }
        if teleport {
            leader
                .snake_head
                .move_head(delta_time as f64, to_snake(leader_pos), MoveMode::Teleport);
        } else {
            let max_distance = delta_time * SPEED;
            let leader_delta = if let Some(p) = cursor_pos {
                let mut v = p - leader_pos;
                let len = v.length();
                if len > max_distance {
                    v *= max_distance / len;
                }
                v
            } else {
                input_dir * max_distance
            };
            leader_pos += leader_delta;
            leader
                .snake_head
                .move_head(delta_time as f64, to_snake(leader_pos), MoveMode::Normal);
        }
        tm.translation = leader_pos;
    }
}

fn follower_move(
    time: Res<Time>,
    mut query_leader: Query<&mut Leader>,
    mut query_tm: Query<&mut Transform>,
) {
    for mut leader in query_leader.iter_mut() {
        let leader = &mut *leader;
        for (body, tm) in leader
            .snake_bodys
            .iter_mut()
            .zip(query_tm.iter_many(&leader.followers))
        {
            body.position = to_snake(tm.translation);
        }
        let delta_time = time.delta_seconds();
        // let delta_time = 1.0 / 60.0;
        leader.snake_head.solve_body(
            &mut leader.snake_bodys,
            delta_time * SPEED,
            delta_time * SPEED * 0.1,
            RADIUS,
        );
        let mut iter_follower_tm = query_tm.iter_many_mut(&leader.followers);
        let mut iter_body = leader.snake_bodys.iter();
        while let (Some(mut tm), Some(body)) = (iter_follower_tm.fetch_next(), iter_body.next()) {
            tm.translation = from_snake(body.position);
        }
        let mut iter_target_tm = query_tm.iter_many_mut(&leader.targets);
        for body in leader.snake_bodys.iter() {
            if let Some(mut tm) = iter_target_tm.fetch_next() {
                tm.translation = from_snake((body.position + body.target) * 0.5);
                if body.target.distance_squared(body.position) > 1.0 {
                    *tm = tm
                        .looking_at(from_snake(body.target), Vec3::Y)
                        .with_scale(Vec3::new(2.0, 1.0, body.target.distance(body.position) * 0.5 + 1.0));
                } else {
                    tm.rotation = Quat::IDENTITY;
                    tm.scale = Vec3::ONE;
                }
                tm.translation += Vec3::new(0.0, RADIUS, 0.0);
            }
        }
    }
}

fn update_path(mut meshes: ResMut<Assets<Mesh>>, query_leader: Query<&Leader>) {
    for leader in query_leader.iter() {
        if let Some(m) = meshes.get_mut(&leader.path_mesh) {
            let poly = LinePoly::from_line(leader.snake_head.get_path(), 1.0);
            if let Some(VertexAttributeValues::Float32x3(pos)) =
                m.attribute_mut(Mesh::ATTRIBUTE_POSITION.id)
            {
                pos.clear();
                pos.extend(poly.vertices.iter().map(|v| v.to_array()));
            }
            if let Some(VertexAttributeValues::Float32x3(nor)) =
                m.attribute_mut(Mesh::ATTRIBUTE_NORMAL.id)
            {
                nor.resize(poly.vertices.len(), [0.0, 0.0, 1.0]);
            }
            if let Some(VertexAttributeValues::Float32x2(uv)) =
                m.attribute_mut(Mesh::ATTRIBUTE_UV_0.id)
            {
                uv.resize(poly.vertices.len(), [0.0, 0.0]);
            }
            if let Some(Indices::U32(ind)) = m.indices_mut() {
                *ind = poly.indices;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    snake_head: SnakeHead,
    snake_bodys: Vec<SnakeBody>,
}

fn save_load(
    keyboard_input: Res<Input<KeyCode>>,
    mut query_leader: Query<(&mut Leader, &mut Transform)>,
    mut query_tm: Query<&mut Transform, Without<Leader>>,
) {
    if keyboard_input.just_pressed(KeyCode::O) {
        let (leader, _) = query_leader.single();
        let data = SaveData {
            snake_head: leader.snake_head.clone(),
            snake_bodys: leader.snake_bodys.clone(),
        };
        let serialized = serde_json::to_string(&data).unwrap();
        std::fs::write("save.json", serialized).unwrap();
    } else if keyboard_input.just_pressed(KeyCode::P) {
        if let Ok(s) = std::fs::read_to_string("save.json") {
            if let Ok(data) = serde_json::from_str::<SaveData>(&s) {
                let (mut leader, mut leader_tm) = query_leader.single_mut();
                leader.snake_head = data.snake_head;
                leader.snake_bodys = data.snake_bodys;
                leader_tm.translation = leader.snake_head.head_position();
                let mut iter_follower_tm = query_tm.iter_many_mut(&leader.followers);
                let mut iter_body = leader.snake_bodys.iter();
                while let (Some(mut tm), Some(body)) =
                    (iter_follower_tm.fetch_next(), iter_body.next())
                {
                    tm.translation = body.position;
                }
            }
        }
    }
}

fn color(i: usize) -> Color {
    let l = if i % 2 == 0 { 0.5 } else { 0.4 };
    Color::hsl(i as f32 * 36.0, 1.0, l)
}

fn setup(
    mut commands: Commands,
    // assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clear_color: ResMut<ClearColor>,
) {
    clear_color.0 = Color::BLACK;
    let mut path_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    path_mesh.set_indices(Some(Indices::U32(Vec::new())));
    path_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    path_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    path_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    let path_mesh = meshes.add(path_mesh);
    commands.spawn((
        PbrBundle {
            mesh: path_mesh.clone(),
            material: materials.add(StandardMaterial::from(Color::GRAY)),
            transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ..default()
        },
        NotShadowCaster,
    ));

    let sphere = meshes.add(shape::Icosphere{radius: RADIUS, subdivisions: 4}.try_into().unwrap());
    let snake_bodys: Vec<_> = (1..10)
        .map(|i| {
            SnakeBody::new(
                i as f32 * 0.1,
                i as f32 * DISTANCE,
                Vec3::new(i as f32 * -DISTANCE, 0.0, 0.0),
            )
        })
        .collect();
    let followers: Vec<_> = snake_bodys
        .iter()
        .enumerate()
        .map(|(i, body)| {
            commands
                .spawn(PbrBundle {
                    mesh: sphere.clone(),
                    material: materials.add(StandardMaterial::from(color(i + 1))),
                    transform: Transform::from_translation(from_snake(body.position)),
                    ..default()
                })
                .id()
        })
        .collect();
    let quad = meshes.add(shape::Plane{size: 2.0, subdivisions: 0}.into());
    let targets: Vec<_> = (0..snake_bodys.len())
        .map(|i| {
            commands
                .spawn((
                    PbrBundle {
                        mesh: quad.clone(),
                        material: materials.add(StandardMaterial::from(color(i + 1))),
                        ..Default::default()
                    },
                    NotShadowCaster,
                ))
                .id()
        })
        .collect();
    let snake_head = SnakeHead::new(
        snake_bodys.last().unwrap().delay * 2.0,
        snake_bodys.last().unwrap().distance * 2.0,
    );
    commands.spawn((
        PbrBundle {
            mesh: sphere.clone(),
            material: materials.add(StandardMaterial::from(color(0))),
            ..default()
        },
        Leader {
            snake_head,
            snake_bodys,
            followers,
            targets,
            path_mesh,
        },
    ));
    let cylinder = meshes.add(shape::Cylinder{radius: RADIUS, height: 10.0, resolution: 16, segments: 1}.into());
    let portals = [
        (0.0, 200.0, 0.0, -200.0),
        (150.0, 180.0, -150.0, -180.0),
        (0.0, -210.0, 200.0, 0.0),
    ];
    for (i, p) in portals.iter().enumerate() {
        let pcolor = Color::hsla(i as f32 * 49.0 + 180.0, 1.0, 0.4, 0.4);
        commands.spawn((
            PbrBundle {
                mesh: cylinder.clone(),
                material: materials.add(StandardMaterial::from(pcolor)),
                transform: Transform::from_translation(from_snake(Vec3::new(p.0, p.1, 0.0))),
                ..default()
            },
            NotShadowCaster,
            Portal(from_snake(Vec3::new(p.2, p.3, 0.0))),
        ));
        commands.spawn((
            PbrBundle {
                mesh: cylinder.clone(),
                material: materials.add(StandardMaterial::from(pcolor)),
                transform: Transform::from_translation(from_snake(Vec3::new(p.2, p.3, 0.0))),
                ..default()
            },
            NotShadowCaster,
        ));
    }
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Plane{size: 2000.0, subdivisions: 0}.into()),
            material: materials.add(StandardMaterial::from(Color::hsl(0.0, 0.0, 0.7))),
            transform: Transform::from_translation(Vec3::new(0.0, -RADIUS, 0.0)),
            ..default()
        },
        NotShadowCaster,
    ));
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight{shadows_enabled: true, ..default()},
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 750.0, 200.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(leader_move)
            .add_system(follower_move.after(leader_move))
            .add_system(update_path)
            .add_system(save_load)
            .add_startup_system(setup);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(SnakePlugin)
        .run();
}
