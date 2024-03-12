mod compute;
mod draw;
mod systems;
mod types;

use bevy::{
    core_pipeline::core_3d::{
        graph::{Core3d, Node3d},
        AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d,
    },
    prelude::*,
    render::{
        render_graph::RenderGraphApp, render_phase::AddRenderCommand, Render, RenderApp, RenderSet,
    },
};

use types::IsosurfaceInstances;

#[derive(Component, Copy, Clone, Debug, PartialEq, Reflect)]
pub struct Isosurface {
    pub radius: f32,
    pub center: Vec3,
    pub grid_size: Vec3,
    pub grid_origin: Vec3,
    // TODO: there is a better way probably...
    //
    // amount of cells in grid is calculated like this
    // x = 8 * grid_density.x
    // y = 8 * grid_density.y
    // z = 8 * grid_density.z
    pub grid_density: UVec3,
}

#[derive(Default)]
pub struct IsosurfacePlugin;

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, systems::insert_fake_mesh);
        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, systems::extract_isosurfaces)
            .add_systems(
                Render,
                (
                    systems::queue_material_isosurfaces::<StandardMaterial>
                        .in_set(RenderSet::Queue),
                    systems::prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    systems::prepare_mesh_uniforms.in_set(RenderSet::PrepareResources),
                    systems::prepare_buffers
                        .in_set(RenderSet::PrepareResources)
                        .after(systems::prepare_mesh_uniforms),
                ),
            )
            .add_render_command::<Transmissive3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Transparent3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Opaque3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<AlphaMask3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .init_resource::<IsosurfaceInstances>()
            .add_render_graph_node::<compute::IsosurfaceComputeNode>(
                Core3d,
                compute::IsosurfaceComputeNodeLabel,
            )
            .add_render_graph_edge(
                Core3d,
                compute::IsosurfaceComputeNodeLabel,
                Node3d::StartMainPass,
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<compute::IsosurfaceComputePipelines>();
    }
}
