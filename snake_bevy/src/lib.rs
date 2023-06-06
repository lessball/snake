#![allow(dead_code)]
pub use bevy::prelude::App;
use bevy::prelude::*;
use bevy::utils::{Duration, Instant};

mod logic;
use logic::*;
mod ground_mesh;
use ground_mesh::GroundMesh;

pub fn init(ground: Option<&str>) -> App {
    let mut app = App::new();
    app.init_resource::<Time>()
        .add_plugin(bevy::core::TaskPoolPlugin::default())
        .add_plugin(SnakeLogicPlugin);
    if let Some(ground) = ground.and_then(GroundMesh::from_obj) {
        app.insert_resource(ground);
    }
    app.setup();
    app
}

pub fn update(
    app: &mut App,
    delta_time: f32,
    input_ray: &[f32],
    input_axis: &[f32],
    position: &mut [f32],
) {
    if let Some(mut time) = app.world.get_resource_mut::<Time>() {
        let t = time
            .last_update()
            .map_or_else(Instant::now, |t| t + Duration::from_secs_f32(delta_time));
        time.update_with_instant(t);
    }
    if let Some(mut m) = app.world.get_resource_mut::<MovementInput>() {
        let ray_dir = Vec3::from_slice(&input_ray[3..6]);
        if ray_dir.length_squared() > 0.0 {
            let ray_origin = Vec3::from_slice(&input_ray[..3]);
            m.ray = Some(Ray {
                origin: ray_origin,
                direction: ray_dir,
            });
        } else {
            m.ray = None;
        }
        m.axis.x = input_axis[0];
        m.axis.y = input_axis[1];
    }
    app.update();
    let (leader, leader_tm) = app
        .world
        .query::<(&Leader, &Transform)>()
        .single(&app.world);
    let followers = leader.followers.clone();
    let mut ipos = position.chunks_mut(3);
    if let Some(p) = ipos.next() {
        p.copy_from_slice(leader_tm.translation.as_ref());
    }
    let mut tm = app.world.query::<&Transform>();
    for (p, f) in ipos.zip(tm.iter_many(&app.world, followers.iter())) {
        p.copy_from_slice(f.translation.as_ref());
    }
}

pub fn get_portals(app: &mut App) -> Box<[f32]> {
    let mut query_portal = app.world.query::<(&Portal, &Transform)>();
    let v: Vec<_> = query_portal
        .iter(&app.world)
        .flat_map(|(p, tm)| {
            [
                tm.translation.x,
                tm.translation.y,
                tm.translation.z,
                p.0.x,
                p.0.y,
                p.0.z,
            ]
        })
        .collect();
    v.into_boxed_slice()
}

pub fn get_path(app: &mut App, path: &mut [f32]) -> u32 {
    let leader = app.world.query::<&Leader>().single(&app.world);
    let mut count = 0;
    for (p0, p1) in leader.snake_head.get_path().zip(path.chunks_mut(3)) {
        p1.copy_from_slice(from_snake(p0).as_ref());
        count += 1;
    }
    count
}

pub fn get_targets(app: &mut App, targets: &mut [f32]) {
    let leader = app.world.query::<&Leader>().single(&app.world);
    for (body, p1) in leader.snake_bodys.iter().zip(targets.chunks_mut(3)) {
        p1.copy_from_slice(from_snake(body.target).as_ref());
    }
}
