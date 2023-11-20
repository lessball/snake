use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::{MeshVertexBufferLayout, PrimitiveTopology, VertexAttributeValues},
        render_resource::{
            AsBindGroup, PolygonMode, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError,
        },
    },
};

use super::logic::*;

#[derive(Component, Default)]
struct Lines();

fn update_lines(
    query_leader: Query<&Leader>,
    query_lines: Query<&Handle<Mesh>, With<Lines>>,
    mut meshes: ResMut<Assets<Mesh>>,
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
        if let Some(mesh) = meshes.get_mut(query_lines.single()) {
            if let Some(VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
            {
                positions.clear();
            }
            if let Some(VertexAttributeValues::Float32x4(colors)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
            {
                colors.clear();
            }
            for leader in query_leader.iter() {
                if show.0 {
                    let mut num = 0;
                    if let Some(VertexAttributeValues::Float32x3(positions)) =
                        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                    {
                        for p in leader.snake_head.get_path() {
                            if positions.len() > 1 {
                                positions.push(*positions.last().unwrap());
                            }
                            positions.push(from_snake(p).to_array());
                        }
                        num = positions.len();
                    }
                    if let Some(VertexAttributeValues::Float32x4(colors)) =
                        mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
                    {
                        for _ in 0..num {
                            colors.push(Color::BLACK.as_rgba_f32());
                        }
                    }
                }
                if show.1 {
                    let iter_tm = query_tm.iter_many(&leader.followers);
                    for (i, (body, tm)) in leader.snake_bodys.iter().zip(iter_tm).enumerate() {
                        if let Some(VertexAttributeValues::Float32x3(positions)) =
                            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                        {
                            positions.push(tm.translation.to_array());
                            positions.push(from_snake(body.target).to_array());
                            if let Some(VertexAttributeValues::Float32x4(colors)) =
                                mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
                            {
                                let color = (super::color(i + 1) * 0.9).as_rgba_f32();
                                colors.push(color);
                                colors.push(color);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {
    let mesh = Mesh::new(PrimitiveTopology::LineList)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(Vec::new()),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_COLOR,
            VertexAttributeValues::Float32x4(Vec::new()),
        );
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(mesh),
            material: materials.add(LineMaterial {}),
            ..default()
        },
        Lines(),
    ));
}

#[derive(Asset, TypePath, Default, AsBindGroup, Debug, Clone)]
pub struct LineMaterial {}

impl Material for LineMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/line_material.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // This is the important part to tell bevy to render this material as a line between vertices
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        Ok(())
    }
}

pub struct LinesPlugin;

impl Plugin for LinesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LineMaterial>::default())
            .add_systems(PostStartup, setup)
            .add_systems(PostUpdate, update_lines);
    }
}
