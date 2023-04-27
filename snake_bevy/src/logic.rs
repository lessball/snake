use bevy::prelude::*;

use super::ground_mesh::GroundMesh;
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
    p.y -= RADIUS;
    for _ in 0..step as i32 {
        p += v;
        p = ground.fix_position(p, precision);
    }
    p.y += RADIUS;
    p
}

pub fn leader_move(
    time: Res<Time>,
    input: Res<MovementInput>,
    mut query_leader: Query<(&mut Leader, &mut Transform)>,
    portal: Query<(&Portal, &Transform), Without<Leader>>,
    ground: Query<&Handle<GroundMesh>>,
    ground_assets: Res<Assets<GroundMesh>>,
) {
    let delta_time = time.delta_seconds();
    let ground = ground_assets.get(ground.single());
    let target = input.ray.map(|ray| {
        ground
            .and_then(|g| g.ray_cast(ray, 999999.0))
            .unwrap_or_else(|| ray.origin - ray.direction * (ray.origin.y / ray.direction.y))
    });
    for (mut leader, mut tm) in query_leader.iter_mut() {
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
        if let Some(ground) = ground {
            leader_pos = move_on_ground(tm.translation, leader_pos, ground);
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
    }
}

pub fn follower_move(
    time: Res<Time>,
    mut query_leader: Query<&mut Leader>,
    mut query_tm: Query<&mut Transform>,
    ground: Query<&Handle<GroundMesh>>,
    ground_assets: Res<Assets<GroundMesh>>,
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
        let ground = ground_assets.get(ground.single());
        let mut iter_follower_tm = query_tm.iter_many_mut(&leader.followers);
        let mut iter_body = leader.snake_bodys.iter();
        while let (Some(mut tm), Some(body)) = (iter_follower_tm.fetch_next(), iter_body.next()) {
            let mut move_to = from_snake(body.position);
            if let Some(g) = ground.as_ref() {
                move_to = move_on_ground(tm.translation, move_to, g);
                if (move_to.y - body.target.y).abs() > RADIUS * 2.0 {
                    // fix different layer
                    let p0 = Vec2::new(move_to.x, move_to.z);
                    let t1 = from_snake(body.target);
                    let p1 = Vec2::new(t1.x, t1.z);
                    if p0.distance_squared(p1) < RADIUS * RADIUS {
                        let ray = Ray {
                            origin: Vec3::new(move_to.x, t1.y - RADIUS * 0.5, move_to.z),
                            direction: Vec3::new(0.0, -1.0, 0.0),
                        };
                        if let Some(p) = g.ray_cast(ray, RADIUS) {
                            move_to = p;
                            move_to.y += RADIUS;
                        }
                    }
                }
            }
            tm.translation = move_to;
        }
    }
}

pub fn setup_logic(mut commands: Commands, asset_server: Res<AssetServer>) {
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
    let snake_head = SnakeHead::new(
        snake_bodys.last().unwrap().delay * 2.0,
        snake_bodys.last().unwrap().distance * 2.0,
    );
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
    commands.spawn(asset_server.load::<GroundMesh, &str>("ground.obj"));
}
