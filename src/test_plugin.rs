use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use super::snake_move::*;

pub struct TestPlugin;

const RADIUS: f32 = 30.0;

#[derive(Component, Default)]
struct Drag {
    offset: Option<Vec2>,
}

fn test_drag(
    windows: Res<Windows>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mousebutton_input: Res<Input<MouseButton>>,
    mut query_drag: Query<(Entity, &mut Drag, &mut Transform)>,
) {
    let cursor_pos = if mousebutton_input.pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera.single();
        let wnd = match camera.target {
            RenderTarget::Window(id) => windows.get(id).unwrap(),
            _ => windows.get_primary().unwrap(),
        };
        wnd.cursor_position().and_then(|pos| {
            camera
                .viewport_to_world(camera_transform, pos)
                .map(|ray| ray.origin.truncate())
        })
    } else {
        None
    };
    let mut select_e = None;
    let mut select_z = f32::MIN;
    let mut select_offset = None;
    for (e, mut drag, mut tm) in query_drag.iter_mut() {
        if let Some(cursor_pos) = cursor_pos {
            if let Some(offset) = drag.offset {
                tm.translation = (cursor_pos + offset).extend(tm.translation.z);
            } else if mousebutton_input.just_pressed(MouseButton::Left) {
                let offset = tm.translation.truncate() - cursor_pos;
                if offset.length_squared() < RADIUS * RADIUS && tm.translation.z > select_z {
                    select_e = Some(e);
                    select_z = tm.translation.z;
                    select_offset = Some(offset);
                }
            }
        } else {
            drag.offset = None;
        }
    }
    if let Some(e) = select_e {
        if let Ok((_, mut drag, _)) = query_drag.get_mut(e) {
            drag.offset = select_offset;
        }
    }
}

#[derive(Component)]
struct TestChr {
    target: Entity,
    ring: Entity,
}

fn test_toi(
    mut query_chr: Query<(&TestChr, &Transform)>,
    mut query_tm: Query<&mut Transform, Without<TestChr>>,
) {
    for [(chr0, tm0), (chr1, tm1)] in query_chr.iter_combinations() {
        let p0 = tm0.translation.truncate();
        let v0 = query_tm.get(chr0.target).map_or(Vec2::ZERO, |tm| tm.translation.truncate() - p0);
        let p1 = tm1.translation.truncate();
        let v1 = query_tm.get(chr1.target).map_or(Vec2::ZERO, |tm| tm.translation.truncate() - p1);
        let t = SnakeHead::toi(p0, v0, p1, v1, RADIUS).unwrap_or(1.0);
        if let Ok(mut ring) = query_tm.get_mut(chr0.ring) {
            ring.translation = (p0 + v0 * t).extend(ring.translation.z);
        }
        if let Ok(mut ring) = query_tm.get_mut(chr1.ring) {
            ring.translation = (p1 + v1 * t).extend(ring.translation.z);
        }
    }
}

fn test_toi_setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
) {
    let ring_texture = assets.load("ring.dds");
    let circle_texture = assets.load("circle.dds");
    for i in 0..2 {
        let fi = i as f32;
        let target = commands
            .spawn((
                SpriteBundle {
                    texture: circle_texture.clone(),
                    transform: Transform::from_translation(Vec3::new(fi * 80.0, 80.0, 1.0 + fi * 0.1)),
                    sprite: Sprite {
                        color: Color::hsla(fi * 72.0, 1.0, 0.5, 0.25),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Drag::default(),
            ))
            .id();
        let ring = commands
            .spawn(SpriteBundle {
                texture: ring_texture.clone(),
                transform: Transform::from_translation(Vec3::new(fi * 80.0, 0.0, 2.0 + fi * 0.1)),
                sprite: Sprite {
                    color: Color::hsl(fi * 72.0, 1.0, 0.25),
                    ..Default::default()
                },
                ..Default::default()
            })
            .id();
        commands.spawn((
            SpriteBundle {
                texture: circle_texture.clone(),
                transform: Transform::from_translation(Vec3::new(fi * 80.0, 0.0, fi * 0.1)),
                sprite: Sprite {
                    color: Color::hsl(fi * 72.0, 1.0, 0.5),
                    ..Default::default()
                },
                ..Default::default()
            },
            Drag::default(),
            TestChr { target, ring },
        ));
    }
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
struct RelaxChr;

fn test_relax(
    mut query_chr: Query<(&RelaxChr, &mut Transform)>,
    query_tm: Query<&Transform, (Without<RelaxChr>, With<Drag>)>,
) {
    let positions: Vec<_> = query_tm
        .iter()
        .map(|tm| tm.translation.truncate())
        .collect();
    let mut pos = Vec2::ZERO;
    let mut escape_dir = Vec2::ZERO;
    let mut escape_range = std::f32::consts::PI;
    for &p in positions.iter() {

    }

    let mut tm = query_chr.single_mut().1;
    tm.translation = pos.extend(0.0);
}

fn test_relax_setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
) {
    let ring_texture = assets.load("ring.dds");
    let circle_texture = assets.load("circle.dds");
    let num = 3;
    for i in 0..num {
        let a = i as f32 / num as f32 * std::f32::consts::PI * 2.0;
        commands.spawn((
            SpriteBundle {
                texture: ring_texture.clone(),
                transform: Transform::from_translation(Vec3::new(60.0 * a.sin(), 60.0 * a.cos(), 1.0 + i as f32 * 0.1)),
                sprite: Sprite {
                    color: Color::hsl(0.0, 0.0, 0.5),
                    ..Default::default()
                },
                ..Default::default()
            },
            Drag::default(),
        ));
    }
    commands.spawn((
        SpriteBundle {
            texture: circle_texture.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            sprite: Sprite {
                color: Color::hsl(0.0, 1.0, 0.5),
                ..Default::default()
            },
            ..Default::default()
        },
        Drag::default(),
        RelaxChr,
    ));
    commands.spawn(
        SpriteBundle {
            texture: ring_texture.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            sprite: Sprite {
                color: Color::hsl(0.0, 1.0, 0.25),
                ..Default::default()
            },
            ..Default::default()
        }
    );
    commands.spawn(Camera2dBundle::default());
}

impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(test_drag)
            .add_system(test_toi)
            .add_startup_system(test_toi_setup);
    }
}
