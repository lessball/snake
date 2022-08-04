use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::camera::RenderTarget;

mod snake_move;
use snake_move::*;

mod line_poly;
use line_poly::LinePoly;

const RADIUS: f32 = 30.0;
const DISTANCE: f32 = 80.0;
const SPEED: f32 = 300.0;

#[derive(Component)]
struct Leader {
    snake_head: SnakeHead,
    followers: Vec<Entity>,
    snake_bodys: Vec<SnakeBody>,
    targets: Vec<Entity>,
    path_mesh: Handle<Mesh>,
}

#[derive(Component)]
struct Follower;

fn leader_move(
    time: Res<Time>,
    windows: Res<Windows>,
    camera: Query<(&Camera, &GlobalTransform)>,
    keyboard_input: Res<Input<KeyCode>>,
    mousebutton_input: Res<Input<MouseButton>>,
    mut query_leader: Query<(&mut Leader, &mut Transform)>,
) {
    let input_dir = IVec2::new(
        keyboard_input.pressed(KeyCode::D) as i32 - keyboard_input.pressed(KeyCode::A) as i32,
        keyboard_input.pressed(KeyCode::W) as i32 - keyboard_input.pressed(KeyCode::S) as i32,
    )
    .as_vec2()
    .normalize_or_zero();
    let cursor_pos = if mousebutton_input.pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera.single();
        let wnd = if let RenderTarget::Window(id) = camera.target {
            windows.get(id).unwrap()
        } else {
            windows.get_primary().unwrap()
        };
        wnd.cursor_position().map(|screen_pos| {
            let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
            let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
            let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
            let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
            world_pos.truncate()
        })
    } else {
        None
    };
    for (mut leader, mut tm) in query_leader.iter_mut() {
        let mut leader_pos = tm.translation.truncate();
        let leader_dir = cursor_pos
            .map(|p| {
                let t = p - leader_pos;
                if t.length_squared() > 1.0 {
                    t.normalize()
                } else {
                    Vec2::ZERO
                }
            })
            .unwrap_or(input_dir);
        let delta_time = time.delta_seconds();
        let leader_delta = leader_dir * (delta_time * SPEED);
        leader_pos += leader_delta;
        tm.translation = leader_pos.extend(0.0);
        leader
            .snake_head
            .move_head(leader_pos, time.seconds_since_startup() as f32);
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
            body.position = tm.translation.truncate();
        }
        let delta_time = time.delta_seconds();
        let target =
            leader
                .snake_head
                .solve_body(&mut leader.snake_bodys, delta_time, SPEED, RADIUS);
        let mut iter_follower_tm = query_tm.iter_many_mut(&leader.followers);
        let mut iter_body = leader.snake_bodys.iter();
        while let (Some(mut tm), Some(body)) = (iter_follower_tm.fetch_next(), iter_body.next()) {
            tm.translation = body.position.extend(0.0);
        }
        let mut iter_target_tm = query_tm.iter_many_mut(&leader.targets);
        let mut iter_target = target.into_iter();
        while let (Some(mut tm), Some(target)) = (iter_target_tm.fetch_next(), iter_target.next()) {
            tm.translation = target.extend(0.0);
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
                pos.extend(poly.vertices.iter().map(|v| [v.x, v.y, 0.0]));
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

fn color(i: usize) -> Color {
    let l = if i % 2 == 0 { 0.5 } else { 0.4 };
    Color::hsl(i as f32 * 36.0, 1.0, l)
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut path_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    path_mesh.set_indices(Some(Indices::U32(Vec::new())));
    path_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    path_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    path_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    let path_mesh = meshes.add(path_mesh);
    commands.spawn_bundle(ColorMesh2dBundle {
        mesh: path_mesh.clone().into(),
        transform: Transform::default(),
        material: materials.add(ColorMaterial::from(Color::GRAY)),
        ..default()
    });

    let sprite_handle = assets.load("ring.dds");
    let cross = assets.load("cross.dds");
    let snake_bodys: Vec<_> = (1..10)
        .map(|i| {
            SnakeBody::new(
                i as f32 * 0.1,
                i as f32 * DISTANCE,
                Vec2::new(i as f32 * -DISTANCE, 0.0),
            )
        })
        .collect();
    let followers: Vec<_> = snake_bodys
        .iter()
        .enumerate()
        .map(|(i, body)| {
            commands
                .spawn_bundle(SpriteBundle {
                    texture: sprite_handle.clone(),
                    transform: Transform::from_translation(body.position.extend(0.0)),
                    sprite: Sprite {
                        color: color(i + 1),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Follower {})
                .id()
        })
        .collect();
    let targets: Vec<_> = snake_bodys
        .iter()
        .enumerate()
        .map(|(i, body)| {
            commands
                .spawn_bundle(SpriteBundle {
                    texture: cross.clone(),
                    transform: Transform::from_translation(body.position.extend(0.0)),
                    sprite: Sprite {
                        color: color(i + 1),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .id()
        })
        .collect();
    let mut snake_head = SnakeHead::new(
        snake_bodys.last().unwrap().delay * 2.0,
        snake_bodys.last().unwrap().distance * 2.0,
    );
    snake_head.reset(Vec2::ZERO, 0.0);
    commands
        .spawn_bundle(SpriteBundle {
            texture: sprite_handle,
            sprite: Sprite {
                color: color(0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Leader {
            snake_head,
            snake_bodys,
            followers,
            targets,
            path_mesh,
        });
    commands.spawn_bundle(Camera2dBundle::default());
}

pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(leader_move)
            .add_system(follower_move)
            .add_system(update_path)
            .add_startup_system(setup);
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_plugin(SnakePlugin)
        .run();
}
