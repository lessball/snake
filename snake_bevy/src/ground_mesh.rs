use bevy::prelude::*;
use parry3d::math::{Isometry, Point, Vector};
use parry3d::query::closest_points::{
    ClosestPoints, CompositeShapeAgainstShapeClosestPointsVisitor,
};
use parry3d::query::DefaultQueryDispatcher;
use parry3d::query::{Ray, RayCast};
use parry3d::shape::{Ball, TriMesh, TypedSimdCompositeShape};

#[derive(Resource)]
pub struct GroundMesh {
    mesh: TriMesh,
}

impl GroundMesh {
    pub fn new(mesh: TriMesh) -> Self {
        Self { mesh }
    }

    pub fn from_obj(data: &str) -> Option<GroundMesh> {
        let mut v: Vec<Point<f32>> = Vec::new();
        let mut ind = Vec::new();
        for line in data.lines() {
            let mut t = line.split(' ');
            match t.next() {
                Some("v") => {
                    let mut a = [0.0; 3];
                    for i in a.iter_mut() {
                        *i = t.next()?.parse().ok()?
                    }
                    v.push(Point::new(a[0], a[1], a[2]));
                }
                Some("f") => {
                    let mut fv = [0; 3];
                    for i in fv.iter_mut() {
                        *i = t.next()?.parse::<u32>().ok()? - 1;
                    }
                    ind.push(fv)
                }
                _ => {}
            }
        }
        let ground_mesh = GroundMesh::new(TriMesh::new(v, ind));
        Some(ground_mesh)
    }    

    pub fn fix_position(&self, mut p: Vec3, d: f32, h: f32) -> Vec3 {
        p.y -= h;
        let ray = Ray::new(Point::new(p.x, p.y + d, p.z), Vector::new(0.0, -1.0, 0.0));
        let mut p1 = p;
        if let Some(t) = self.mesh.cast_local_ray(&ray, d * 2.0, false) {
            p1 = Vec3::new(ray.origin.x, ray.origin.y - t, ray.origin.z);
        } else {
            let ball = Ball::new(0.001);
            let pos12 = Isometry::translation(p.x, p.y, p.z);
            let dispatcher = DefaultQueryDispatcher;
            let mut visitor = CompositeShapeAgainstShapeClosestPointsVisitor::new(
                &dispatcher,
                &pos12,
                &self.mesh,
                &ball,
                d,
            );
            if let Some((_, (_, ClosestPoints::WithinMargin(p2, _)))) =
                self.mesh.typed_qbvh().traverse_best_first(&mut visitor)
            {
                p1 = Vec3::new(p2.x, p2.y, p2.z);
            };
        }
        p1.y += h;
        p1
    }

    pub fn ray_cast(&self, ray: bevy::prelude::Ray, d: f32) -> Option<Vec3> {
        let ray = Ray::new(
            Point::new(ray.origin.x, ray.origin.y, ray.origin.z),
            Vector::new(ray.direction.x, ray.direction.y, ray.direction.z),
        );
        self.mesh.cast_local_ray(&ray, d, false).map(|d| {
            let p = ray.point_at(d);
            Vec3::new(p.x, p.y, p.z)
        })
    }
}
