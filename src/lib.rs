mod compute;
mod draw;
mod systems;
mod types;

use bevy::{
    core_pipeline::{
        core_3d::{
            graph::{Core3d, Node3d},
            AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d,
        },
        deferred::{AlphaMask3dDeferred, Opaque3dDeferred},
    },
    pbr::Shadow,
    prelude::*,
    render::{
        batching::batch_and_prepare_render_phase, render_graph::RenderGraphApp,
        render_phase::AddRenderCommand, render_resource::SpecializedMeshPipelines, Render,
        RenderApp, RenderSet,
    },
};

use draw::IsosurfaceMaterialPipeline;
use types::IsosurfaceInstances;

#[derive(Component, Copy, Clone, Debug, PartialEq, Reflect)]
pub struct Isosurface {
    pub radius: f32,
    pub center: Vec3,
    // TODO: hide from user
    pub fake_mesh_asset: AssetId<Mesh>,
}

#[derive(Component, Copy, Clone, Debug, PartialEq, Reflect)]
pub struct Polygonization {
    pub grid_size: Vec3,
    pub grid_origin: Vec3,
}

#[derive(Default)]
pub struct IsosurfacePlugin;

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, systems::extract_isosurfaces)
            .add_systems(
                Render,
                (
                    systems::queue_material_isosurfaces::<StandardMaterial>.in_set(RenderSet::Queue),
                    systems::prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    (
                        batch_and_prepare_render_phase::<Opaque3d, IsosurfaceMaterialPipeline<StandardMaterial>>,
                        batch_and_prepare_render_phase::<Transmissive3d, IsosurfaceMaterialPipeline<StandardMaterial>>,
                        batch_and_prepare_render_phase::<Transparent3d, IsosurfaceMaterialPipeline<StandardMaterial>>,
                        batch_and_prepare_render_phase::<AlphaMask3d, IsosurfaceMaterialPipeline<StandardMaterial>>,
                        batch_and_prepare_render_phase::<Shadow, IsosurfaceMaterialPipeline<StandardMaterial>>,
                        batch_and_prepare_render_phase::<Opaque3dDeferred, IsosurfaceMaterialPipeline<StandardMaterial>>,
                        batch_and_prepare_render_phase::<AlphaMask3dDeferred, IsosurfaceMaterialPipeline<StandardMaterial>>,
                    ).in_set(RenderSet::PrepareResources),
                    (
                        systems::fill_batch_data::<Opaque3d>.after(batch_and_prepare_render_phase::<Opaque3d, IsosurfaceMaterialPipeline<StandardMaterial>>),
                        systems::fill_batch_data::<Transmissive3d>.after(batch_and_prepare_render_phase::<Transmissive3d, IsosurfaceMaterialPipeline<StandardMaterial>>),
                        systems::fill_batch_data::<Transparent3d>.after(batch_and_prepare_render_phase::<Transparent3d, IsosurfaceMaterialPipeline<StandardMaterial>>),
                        systems::fill_batch_data::<AlphaMask3d>.after(batch_and_prepare_render_phase::<AlphaMask3d, IsosurfaceMaterialPipeline<StandardMaterial>>),
                        systems::fill_batch_data::<Shadow>.after(batch_and_prepare_render_phase::<Shadow, IsosurfaceMaterialPipeline<StandardMaterial>>),
                        systems::fill_batch_data::<Opaque3dDeferred>.after(batch_and_prepare_render_phase::<Opaque3dDeferred, IsosurfaceMaterialPipeline<StandardMaterial>>),
                        systems::fill_batch_data::<AlphaMask3dDeferred>.after(batch_and_prepare_render_phase::<AlphaMask3dDeferred, IsosurfaceMaterialPipeline<StandardMaterial>>),
                    ).in_set(RenderSet::PrepareResources),
                    systems::prepare_buffers.in_set(RenderSet::PrepareResources)
                        .after(systems::fill_batch_data::<Opaque3d>)
                        .after(systems::fill_batch_data::<Transmissive3d>)
                        .after(systems::fill_batch_data::<Transparent3d>)
                        .after(systems::fill_batch_data::<AlphaMask3d>)
                        .after(systems::fill_batch_data::<Shadow>)
                        .after(systems::fill_batch_data::<Opaque3dDeferred>)
                        .after(systems::fill_batch_data::<AlphaMask3dDeferred>)
                ),
            )
            .add_render_command::<Transmissive3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Transparent3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Opaque3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<AlphaMask3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .init_resource::<SpecializedMeshPipelines<draw::IsosurfaceMaterialPipeline<StandardMaterial>>>()
            .init_resource::<IsosurfaceInstances>()
            .add_render_graph_node::<compute::IsosurfaceComputeNode>(Core3d, compute::IsosurfaceComputeNodeLabel)
            .add_render_graph_edge(Core3d, compute::IsosurfaceComputeNodeLabel, Node3d::StartMainPass);
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<draw::IsosurfaceMaterialPipeline<StandardMaterial>>()
            .init_resource::<compute::IsosurfaceComputePipeline>();
    }
}
