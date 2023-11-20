use bevy::prelude::*;

use super::ground_mesh::GroundMesh;
// use super::character_move::character_move;
use snake_move::*;

pub fn to_snake(v: Vec3) -> Vec3 {
    Vec3::new(v.x, -v.z, v.y)
}

pub fn from_snake(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.z, -v.y)
}

pub const RADIUS: f32 = 30.0;
pub const DISTANCE: f32 = 80.0;
pub const SPEED: f32 = 300.0;

#[derive(Component)]
pub struct Portal(pub Vec3);

#[derive(Component)]
pub struct Leader {
    pub snake_head: SnakeHead,
    pub snake_bodys: Vec<SnakeBody>,
    pub followers: Vec<Entity>,
}

#[derive(Resource, Default)]
pub struct MovementInput {
    pub ray: Option<Ray>,
    pub axis: Vec2,
}

fn move_on_ground(from: Vec3, to: Vec3, ground: &GroundMesh) -> Vec3 {
    let precision = 3.0;
    let mut v = to - from;
    let step = (v.length() / precision).floor() + 1.0;
    v /= step;
    let mut p = from;
    for _ in 0..step as i32 {
        p += v;
        p = ground.fix_position(p, precision, RADIUS);
    }
    p
}

fn leader_move(
    time: Res<Time>,
    input: Res<MovementInput>,
    ground: Option<Res<GroundMesh>>,
    mut query_leader: Query<(&mut Leader, &mut Transform)>,
    portal: Query<(&Portal, &Transform), Without<Leader>>,
) {
    let delta_time = time.delta_seconds();
    let ground = ground.as_ref();
    let target = input.ray.map(|ray| {
        ground
            .and_then(|g| g.ray_cast(ray, 999999.0))
            .unwrap_or_else(|| ray.origin - ray.direction * (ray.origin.y / ray.direction.y))
    });
    query_leader
        .par_iter_mut()
        .for_each(|(mut leader, mut tm)| {
            let mut leader_pos = tm.translation;
            let mut teleport = false;
            for (pt, tm) in portal.iter() {
                if tm.translation.distance_squared(leader_pos) < RADIUS * RADIUS {
                    leader_pos = pt.0;
                    teleport = true;
                    break;
                }
            }
            if !teleport {
                let max_distance = delta_time * SPEED;
                let move_delta = if let Some(p) = target {
                    let mut v = p - leader_pos;
                    v.y = 0.0;
                    let len = v.length();
                    if len > max_distance {
                        v *= max_distance / len;
                    }
                    v
                } else {
                    Vec3::new(input.axis.x, 0.0, -input.axis.y) * max_distance
                };
                leader_pos += move_delta;
            }
            if let Some(g) = ground {
                if !teleport {
                    leader_pos = move_on_ground(tm.translation, leader_pos, g);
                    // leader_pos = character_move(tm.translation, leader_pos, RADIUS, &g.mesh, 1.5, false);
                } else {
                    leader_pos = g.fix_position(leader_pos, 3.0, RADIUS);
                }
            }
            leader.snake_head.move_head(
                delta_time as f64,
                to_snake(leader_pos),
                if teleport {
                    MoveMode::Teleport
                } else {
                    MoveMode::Normal
                },
            );
            tm.translation = leader_pos;
        });
}

fn follower_move(
    time: Res<Time>,
    ground: Option<Res<GroundMesh>>,
    mut query_leader: Query<&mut Leader>,
    mut query_tm: Query<&mut Transform>,
) {
    let delta_time = time.delta_seconds();
    // let delta_time = 1.0 / 60.0;
    query_leader.par_iter_mut().for_each(|mut leader| {
        let leader = &mut *leader;
        for (body, tm) in leader
            .snake_bodys
            .iter_mut()
            .zip(query_tm.iter_many(&leader.followers))
        {
            body.position = to_snake(tm.translation);
        }
        let fix_position = ground.as_ref().map(|g| {
            |_body: &SnakeBody, pos, prev| {
                to_snake(move_on_ground(from_snake(prev), from_snake(pos), g))
            }
        });
        leader
            .snake_head
            .update_body(&mut leader.snake_bodys, RADIUS);
        leader.snake_head.solve_body(
            &mut leader.snake_bodys,
            delta_time * SPEED,
            delta_time * SPEED * 0.1,
            RADIUS,
            fix_position,
        );
        if let Some(g) = ground.as_ref() {
            for body in leader.snake_bodys.iter_mut() {
                let mut pos = from_snake(body.position);
                if (pos.y - body.target.y).abs() > RADIUS * 2.0 {
                    // fix different layer
                    let p0 = Vec2::new(pos.x, pos.z);
                    let t1 = from_snake(body.target);
                    let p1 = Vec2::new(t1.x, t1.z);
                    if p0.distance_squared(p1) < RADIUS * RADIUS {
                        let ray = Ray {
                            origin: Vec3::new(pos.x, t1.y - RADIUS * 0.5, pos.z),
                            direction: Vec3::new(0.0, -1.0, 0.0),
                        };
                        if let Some(p) = g.ray_cast(ray, RADIUS) {
                            pos = p;
                            pos.y += RADIUS;
                        }
                    }
                } else {
                    // fix stuck
                    let target = from_snake(body.target);
                    let v = target - pos;
                    let len2 = v.length_squared();
                    if len2 > RADIUS * RADIUS * 64.0 {
                        let pos1 = pos + v * (3.0 / len2.sqrt());
                        let pos2 = g.fix_position(pos1, 3.0, RADIUS);
                        if pos2.distance_squared(pos) < 0.1 {
                            pos = target;
                        }
                    }
                }
                body.position = to_snake(pos);
            }
        }
    });
    for leader in query_leader.iter() {
        let mut iter_follower_tm = query_tm.iter_many_mut(&leader.followers);
        let mut iter_body = leader.snake_bodys.iter();
        while let (Some(mut tm), Some(body)) = (iter_follower_tm.fetch_next(), iter_body.next()) {
            tm.translation = from_snake(body.position);
        }
    }
}

fn setup_logic(mut commands: Commands) {
    let snake_bodys: Vec<_> = (1..10)
        .map(|i| {
            SnakeBody::new(
                i as f32 * 0.1,
                i as f32 * DISTANCE,
                Vec3::new(i as f32 * -DISTANCE, 0.0, RADIUS),
            )
        })
        .collect();
    let followers: Vec<_> = snake_bodys
        .iter()
        .map(|body| {
            commands
                .spawn(Transform::from_translation(from_snake(body.position)))
                .id()
        })
        .collect();
    let snake_head = SnakeHead::new();
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, RADIUS, 0.0)),
        Leader {
            snake_head,
            snake_bodys,
            followers,
        },
    ));
    let portals = [
        (0.0, 200.0, 0.0, -200.0),
        (150.0, 180.0, -150.0, -180.0),
        (0.0, -210.0, 200.0, 0.0),
    ];
    for p in portals.iter() {
        commands.spawn((
            Transform::from_translation(from_snake(Vec3::new(p.0, p.1, RADIUS))),
            Portal(from_snake(Vec3::new(p.2, p.3, RADIUS))),
        ));
    }
}

pub struct SnakeLogicPlugin;

impl Plugin for SnakeLogicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MovementInput>()
            .add_systems(Startup, setup_logic)
            .add_systems(Update, (leader_move, follower_move.after(leader_move)));
    }
}
