use bevy::prelude::*;

use super::ground_mesh::GroundMesh;
// use super::character_move::character_move;
use snake_move::*;

use std::iter;

pub fn to_snake(v: Vec3) -> Vec3 {
    Vec3::new(v.x, -v.z, v.y)
}

pub fn from_snake(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.z, -v.y)
}

pub const RADIUS: f32 = 30.0;
pub const DISTANCE: f32 = 80.0;
pub const SPEED: f32 = 300.0;

fn get_delay(i: usize) -> f32 {
    i as f32 * 0.1
}

fn get_distance(i: usize) -> f32 {
    i as f32 * DISTANCE
}

#[derive(Component)]
pub struct Portal(pub Vec3);

#[derive(Component)]
pub struct Leader {
    pub snake_head: SnakeHead,
    pub followers: Vec<Entity>,
    stack_state: i32,
    stack_time: f64,
    stack_pos: Vec3,
    head_dir: Vec3,
}

impl Leader {
    fn new(snake_bodies: Vec<SnakeBody>, followers: Vec<Entity>) -> Self {
        Self {
            snake_head: SnakeHead::new(snake_bodies),
            followers,
            stack_state: 0,
            stack_time: 0.0,
            stack_pos: Vec3::ZERO,
            head_dir: Vec3::X,
        }
    }

    fn can_move(&self) -> bool {
        self.stack_state == 0
    }

    fn stack_up(&mut self) {
        if self.stack_state == 0 && self.snake_head.bodies.len() > 1 {
            self.stack_state = 1;
            self.stack_time = 0.1;
            self.snake_head.trim_head(1);
            self.stack_pos = self.snake_head.bodies.remove(0).position;
            for (i, body) in self.snake_head.bodies.iter_mut().enumerate() {
                body.delay = get_delay(i);
                body.distance = get_distance(i);
            }
        }
    }

    fn stack_down(&mut self) {
        if self.stack_state == 0 && self.snake_head.bodies.len() <= self.followers.len() {
            self.stack_state = -1;
            self.stack_time = 0.1;
            self.stack_pos =
                self.snake_head.bodies[0].position + to_snake(self.head_dir * DISTANCE);
            self.snake_head
                .bodies
                .insert(0, self.snake_head.bodies[0].clone());
            self.snake_head.bodies[0].collision = false;
            for (i, body) in self.snake_head.bodies.iter_mut().enumerate() {
                body.delay = get_delay(i);
                body.distance = get_distance(i);
            }
        }
    }

    fn update_stack(&mut self, delta_time: f64) {
        let mut head_pos = self.snake_head.head_position();
        match self.stack_state {
            1 => {
                if self.stack_time > delta_time {
                    head_pos = head_pos.lerp(self.stack_pos, (delta_time / self.stack_time) as f32);
                    self.stack_time -= delta_time;
                } else {
                    head_pos = self.stack_pos;
                    self.stack_time = 0.0;
                    self.stack_state = 0;
                }
            }
            -1 => {
                if self.stack_time > delta_time {
                    head_pos = head_pos.lerp(self.stack_pos, (delta_time / self.stack_time) as f32);
                    self.stack_time -= delta_time;
                } else {
                    head_pos = self.stack_pos;
                    self.stack_time = 0.0;
                    self.stack_state = 0;
                    self.snake_head.bodies[0].collision = true;
                }
            }
            _ => {}
        }
        self.snake_head
            .move_head(delta_time, head_pos, MoveMode::Normal);
    }

    fn entity_position(&self, i: usize) -> Vec3 {
        let offset = self.followers.len() + 1 - self.snake_head.bodies.len();
        if i > offset {
            from_snake(self.snake_head.bodies[i - offset].position)
        } else {
            let mut pos = from_snake(self.snake_head.head_position());
            match self.stack_state {
                1 => {
                    if i < offset {
                        pos = from_snake(self.stack_pos);
                        pos.y -= self.stack_time as f32 / 0.1 * RADIUS * 2.0;
                    }
                }
                -1 => {
                    pos.y += self.stack_time as f32 / 0.1 * RADIUS * 2.0;
                }
                _ => {}
            }
            pos.y += (offset - i) as f32 * RADIUS * 2.0;
            pos
        }
    }
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
    mut query_leader: Query<&mut Leader>,
    portal: Query<(&Portal, &Transform)>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    let delta_time = time.delta_seconds();
    let ground = ground.as_ref();
    let target = input.ray.map(|ray| {
        ground
            .and_then(|g| g.ray_cast(ray, 999999.0))
            .unwrap_or_else(|| ray.origin - ray.direction * (ray.origin.y / ray.direction.y))
    });
    query_leader.par_iter_mut().for_each(|mut leader| {
        if leader.can_move() {
            if keyboard_input.pressed(KeyCode::U) {
                leader.stack_up();
            } else if keyboard_input.pressed(KeyCode::J) {
                leader.stack_down();
            }
        }
        if leader.can_move() {
            let mut leader_pos = from_snake(leader.snake_head.head_position());
            let start_pos = leader_pos.clone();
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
                if let Some(dir) = move_delta.try_normalize() {
                    leader.head_dir = dir;
                }
            }
            if let Some(g) = ground {
                if !teleport {
                    leader_pos = move_on_ground(start_pos, leader_pos, g);
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
        } else {
            leader.update_stack(delta_time as f64);
        }
    });
}

fn body_move(
    time: Res<Time>,
    ground: Option<Res<GroundMesh>>,
    mut query_leader: Query<(&mut Leader, Entity)>,
    mut query_tm: Query<&mut Transform>,
) {
    let delta_time = time.delta_seconds();
    // let delta_time = 1.0 / 60.0;
    query_leader.par_iter_mut().for_each(|(mut leader, _)| {
        let leader = &mut *leader;
        let fix_position = ground.as_ref().map(|g| {
            |_body: &SnakeBody, pos, prev| {
                to_snake(move_on_ground(from_snake(prev), from_snake(pos), g))
            }
        });
        leader.snake_head.update_body(RADIUS);
        leader.snake_head.solve_body(
            delta_time * SPEED,
            delta_time * SPEED * 0.1,
            RADIUS,
            fix_position,
        );
        if let Some(g) = ground.as_ref() {
            for body in leader.snake_head.bodies.iter_mut().skip(1) {
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
    for (leader, entity) in query_leader.iter() {
        let iter_entity = iter::once(&entity).chain(leader.followers.iter());
        let mut iter_tm = query_tm.iter_many_mut(iter_entity);
        let mut i = 0;
        while let Some(mut tm) = iter_tm.fetch_next() {
            tm.translation = leader.entity_position(i);
            i += 1;
        }
    }
}

fn setup_logic(mut commands: Commands) {
    let snake_bodies: Vec<_> = (0..10)
        .map(|i| {
            SnakeBody::new(
                get_delay(i),
                get_distance(i),
                Vec3::new(-get_distance(i), 0.0, RADIUS),
            )
        })
        .collect();
    let followers: Vec<_> = snake_bodies
        .iter()
        .skip(1)
        .map(|body| {
            commands
                .spawn(Transform::from_translation(from_snake(body.position)))
                .id()
        })
        .collect();
    let head_pos = from_snake(snake_bodies[0].position);
    commands.spawn((
        Transform::from_translation(head_pos),
        Leader::new(snake_bodies, followers),
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
            .add_systems(Update, (leader_move, body_move.after(leader_move)));
    }
}
