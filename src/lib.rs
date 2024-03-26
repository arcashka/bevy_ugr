mod assets;
mod compute;
mod draw;
mod systems;
mod types;

use assets::IsosurfaceAssetsPlugin;
use bevy::{
    core_pipeline::core_3d::{
        graph::{Core3d, Node3d},
        AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d,
    },
    prelude::*,
    render::{
        batching::batch_and_prepare_render_phase, render_graph::RenderGraphApp,
        render_phase::AddRenderCommand, Render, RenderApp, RenderSet,
    },
};

use compute::{
    BuildIndirectBufferBindGroups, CalculateIsosurfaceBindGroups, CalculateIsosurfaceTasks,
    IndirectBuffersCollection,
};
use draw::{DrawBindGroups, IsosurfaceBatcher};
use systems::{prepare_model_bind_group_layout, queue_material_isosurfaces};
use types::{IsosurfaceBuffersCollection, IsosurfaceInstances, PrepareIndirects};

pub use assets::IsosurfaceAsset;

#[derive(Default)]
pub struct IsosurfacePlugin;

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, systems::insert_fake_mesh)
            .add_plugins(IsosurfaceAssetsPlugin);

        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, systems::extract_isosurfaces)
            .add_systems(
                Render,
                (
                    (
                        queue_material_isosurfaces::<StandardMaterial>,
                        compute::queue_isosurface_calculations,
                    )
                        .in_set(RenderSet::Queue),
                    (
                        batch_and_prepare_render_phase::<Transmissive3d, IsosurfaceBatcher>,
                        batch_and_prepare_render_phase::<Transparent3d, IsosurfaceBatcher>,
                        batch_and_prepare_render_phase::<Opaque3d, IsosurfaceBatcher>,
                        batch_and_prepare_render_phase::<AlphaMask3d, IsosurfaceBatcher>,
                        //     .before(queue_prepare_indirects),
                        // queue_prepare_indirects,
                    )
                        // Usually batch_and_prepare_render_phase is called in PrepareResources
                        // set. But we need to read the instancing info generated there and write
                        // it to the indirect buffer that's why it's moved to PhaseSort
                        .in_set(RenderSet::PhaseSort),
                    (
                        compute::prepare_calculation_buffers,
                        compute::prepare_indirect_buffers,
                        prepare_model_bind_group_layout,
                    )
                        .in_set(RenderSet::PrepareResources),
                    (
                        compute::check_calculate_isosurfaces_for_readiness,
                        compute::prepare_calculate_isosurface_bind_groups,
                        compute::prepare_generate_indirect_buffer_bind_groups,
                    )
                        .in_set(RenderSet::PrepareBindGroups),
                    compute::cleanup_calculated_isosurface.in_set(RenderSet::Cleanup),
                ),
            )
            .add_render_command::<Transmissive3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Transparent3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Opaque3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<AlphaMask3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .init_resource::<CalculateIsosurfaceTasks>()
            .init_resource::<IsosurfaceInstances>()
            .init_resource::<DrawBindGroups>()
            .init_resource::<PrepareIndirects>()
            .init_resource::<IndirectBuffersCollection>()
            .init_resource::<IsosurfaceBuffersCollection>()
            .init_resource::<CalculateIsosurfaceBindGroups>()
            .init_resource::<BuildIndirectBufferBindGroups>()
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
