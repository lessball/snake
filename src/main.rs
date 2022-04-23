use bevy::prelude::*;

mod snake_move;
use snake_move::*;

const RADIUS: f32 = 30.0;
const DISTANCE: f32 = 80.0;
const SPEED: f32 = 300.0;

#[derive(Component)]
struct Leader {
    snake_head: SnakeHead,
    followers: Vec<Entity>,
    snake_bodys: Vec<SnakeBody>,
}

impl Leader {
    fn update_body<F>(&mut self, f: F)
    where
        F: Fn(&Entity, &mut SnakeBody),
    {
        for (entity, body) in self.followers.iter().zip(self.snake_bodys.iter_mut()) {
            f(entity, body);
        }
    }
    fn solve_body(&mut self, step_time: f32) {
        self.snake_head
            .solve_body(&mut self.snake_bodys, step_time, SPEED, RADIUS);
    }
}

#[derive(Component)]
struct Follower;

fn leader_move(
    time: Res<Time>,
    windows: Res<Windows>,
    keyboard_input: Res<Input<KeyCode>>,
    mousebutton_input: Res<Input<MouseButton>>,
    mut query_leader: Query<(&mut Leader, &mut Transform)>,
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
    for (mut leader, mut tm) in query_leader.iter_mut() {
        let mut leader_pos = tm.translation.truncate();
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
        tm.translation = leader_pos.extend(0.0);
        leader
            .snake_head
            .move_head(leader_pos, time.seconds_since_startup() as f32);
    }
}

fn follower_move(
    time: Res<Time>,
    mut query_leader: Query<&mut Leader>,
    mut query_follower: Query<&mut Transform, With<Follower>>,
) {
    for mut leader in query_leader.iter_mut() {
        leader.update_body(|entity, body| {
            if let Ok(tm) = query_follower.get(*entity) {
                body.position = tm.translation.truncate();
            }
        });
        let delta_time = time.delta_seconds();
        let step_count = (delta_time * 300.0).floor().max(1.0).min(5.0);
        let step_time = delta_time / step_count;
        for _ in 0..step_count as i32 {
            leader.solve_body(step_time);
        }
        for (entity, body) in leader.followers.iter().zip(leader.snake_bodys.iter()) {
            if let Ok(mut tm) = query_follower.get_mut(*entity) {
                tm.translation = body.position.extend(0.0);
            }
        }
        // if let Ok(tm) = query_follower.get(leader.followers[0]) {
        //     println!("{}", tm.translation.x);
        // }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let sprite_handle = assets.load("circle.png");
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
                        color: Color::hsl((i + 1) as f32 * 36.0, 1.0, 0.5),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Follower {})
                .id()
        })
        .collect();
    let mut snake_head = SnakeHead::new();
    snake_head.reset(Vec2::ZERO, 0.0);
    commands
        .spawn_bundle(SpriteBundle {
            texture: sprite_handle,
            sprite: Sprite {
                color: Color::hsl(0.0, 1.0, 0.5),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Leader {
            snake_head,
            snake_bodys,
            followers,
        });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(leader_move)
            .add_system(follower_move)
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
