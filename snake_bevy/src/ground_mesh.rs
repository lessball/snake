use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use parry3d::math::{Isometry, Point, Vector};
use parry3d::query::closest_points::{
    ClosestPoints, CompositeShapeAgainstShapeClosestPointsVisitor,
};
use parry3d::query::DefaultQueryDispatcher;
use parry3d::query::{Ray, RayCast};
use parry3d::shape::{Ball, TriMesh, TypedSimdCompositeShape};

#[derive(TypeUuid)]
#[uuid = "38dfdb54-6b1a-4ea2-8cba-c44a641af4d6"]
pub struct GroundMesh {
    mesh: TriMesh,
}

impl GroundMesh {
    pub fn new(mesh: TriMesh) -> Self {
        Self { mesh }
    }

    pub fn fix_position(&self, p: Vec3, d: f32) -> Vec3 {
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
        p1
    }
}
