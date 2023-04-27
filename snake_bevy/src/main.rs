use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy_prototype_debug_lines::{ DebugLines, DebugLinesPlugin };
use serde::{Deserialize, Serialize};

use snake_move::*;

mod logic;
use logic::*;
mod ground_mesh;
mod obj_ground_loader;

fn movement_input(
    mut movement_input: ResMut<MovementInput>,
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    keyboard_input: Res<Input<KeyCode>>,
    mousebutton_input: Res<Input<MouseButton>>,
) {
    movement_input.axis = Vec2::new(
        keyboard_input.pressed(KeyCode::D) as i32 as f32
            - keyboard_input.pressed(KeyCode::A) as i32 as f32,
        keyboard_input.pressed(KeyCode::W) as i32 as f32
            - keyboard_input.pressed(KeyCode::S) as i32 as f32,
    )
    .normalize_or_zero();
    movement_input.ray = if mousebutton_input.pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera.single();
        let wnd = window.single();
        wnd.cursor_position()
            .and_then(|pos| camera.viewport_to_world(camera_transform, pos))
    } else {
        None
    };
}

fn update_lines(
    query_leader: Query<&Leader>,
    mut lines: ResMut<DebugLines>,
    query_tm: Query<&Transform>,
    keyboard_input: Res<Input<KeyCode>>,
    mut show: Local<(bool, bool)>,
) {
    if keyboard_input.just_pressed(KeyCode::P) {
        show.0 = !show.0;
    }
    if keyboard_input.just_pressed(KeyCode::T) {
        show.1 = !show.1;
    }
    if show.0 || show.1 {
        for leader in query_leader.iter() {
            if show.0 {
                let mut iter_path = leader.snake_head.get_path().map(from_snake);
                if let Some(mut p0) = iter_path.next() {
                    for p1 in iter_path {
                        lines.line_colored(p0, p1, 0.0, Color::GRAY);
                        p0 = p1;
                    }
                }
            }
            if show.1 {
                let iter_tm = query_tm.iter_many(&leader.followers);
                for (i, (body, tm)) in leader.snake_bodys.iter().zip(iter_tm).enumerate() {
                    lines.line_colored(tm.translation, from_snake(body.target), 0.0, color(i + 1) * 0.9);
                }
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
    if keyboard_input.just_pressed(KeyCode::Z) {
        let (leader, _) = query_leader.single();
        let data = SaveData {
            snake_head: leader.snake_head.clone(),
            snake_bodys: leader.snake_bodys.clone(),
        };
        let serialized = serde_json::to_string(&data).unwrap();
        std::fs::write("save.json", serialized).unwrap();
    } else if keyboard_input.just_pressed(KeyCode::X) {
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

fn setup_render(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clear_color: ResMut<ClearColor>,
    query_leader: Query<(&Leader, Entity)>,
    query_portal: Query<(&Portal, Entity)>,
    query_tm: Query<&Transform>,
) {
    clear_color.0 = Color::BLACK;

    let sphere = meshes.add(
        shape::Icosphere {
            radius: RADIUS,
            subdivisions: 4,
        }
        .try_into()
        .unwrap(),
    );
    for (leader, leader_entity) in query_leader.iter() {
        commands.entity(leader_entity).insert(PbrBundle {
            mesh: sphere.clone(),
            material: materials.add(StandardMaterial::from(color(0))),
            transform: *query_tm.get(leader_entity).unwrap(),
            ..default()
        });
        for (i, tm) in query_tm.iter_many(&leader.followers).enumerate() {
            commands.entity(leader.followers[i]).insert(PbrBundle {
                mesh: sphere.clone(),
                material: materials.add(StandardMaterial::from(color(i + 1))),
                transform: *tm,
                ..default()
            });
        }
    }
    let cylinder = meshes.add(
        shape::Cylinder {
            radius: RADIUS,
            height: 10.0,
            resolution: 16,
            segments: 1,
        }
        .into(),
    );
    for (i, (portal, portal_entity)) in query_portal.iter().enumerate() {
        let pcolor = Color::hsla(i as f32 * 49.0 + 180.0, 1.0, 0.4, 0.4);
        commands.entity(portal_entity).insert((
            PbrBundle {
                mesh: cylinder.clone(),
                material: materials.add(StandardMaterial::from(pcolor)),
                transform: *query_tm.get(portal_entity).unwrap(),
                ..default()
            },
            NotShadowCaster,
        ));
        commands.spawn((
            PbrBundle {
                mesh: cylinder.clone(),
                material: materials.add(StandardMaterial::from(pcolor)),
                transform: Transform::from_translation(portal.0),
                ..default()
            },
            NotShadowCaster,
        ));
    }
    commands.spawn(SceneBundle {
        scene: asset_server.load("ground.glb#Scene0"),
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -1.5, -1.5, 0.0)),
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
        app.add_asset::<ground_mesh::GroundMesh>()
            .init_resource::<MovementInput>()
            .init_asset_loader::<obj_ground_loader::ObjGroundLoader>()
            .add_system(movement_input)
            .add_system(leader_move.after(movement_input))
            .add_system(follower_move.after(leader_move))
            .add_system(update_lines.after(follower_move))
            .add_system(save_load)
            .add_startup_system(setup_logic)
            .add_startup_system(setup_render.in_base_set(StartupSet::PostStartup));
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(SnakePlugin)
        .add_plugin(DebugLinesPlugin::default())
        .run();
}
