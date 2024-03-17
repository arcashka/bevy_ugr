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
        render_graph::RenderGraphApp, render_phase::AddRenderCommand, Render, RenderApp, RenderSet,
    },
};

use compute::{CalculateIsosurfaces, IsosurfaceBindGroupsCollection};
use draw::DrawBindGroups;
use systems::{prepare_bind_group, prepare_mesh_uniforms, queue_material_isosurfaces};
use types::{IsosurfaceBuffersCollection, IsosurfaceIndicesCollection, IsosurfaceInstances};

pub use assets::Isosurface;

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
                    queue_material_isosurfaces::<StandardMaterial>.in_set(RenderSet::Queue),
                    compute::queue_isosurface_calculations.in_set(RenderSet::Queue),
                    compute::prepare_buffers
                        .in_set(RenderSet::PrepareResources)
                        .after(systems::prepare_mesh_uniforms),
                    compute::prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    prepare_mesh_uniforms.in_set(RenderSet::PrepareResources),
                    prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .add_render_command::<Transmissive3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Transparent3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Opaque3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<AlphaMask3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .init_resource::<CalculateIsosurfaces>()
            .init_resource::<IsosurfaceInstances>()
            .init_resource::<DrawBindGroups>()
            .init_resource::<IsosurfaceIndicesCollection>()
            .init_resource::<IsosurfaceBuffersCollection>()
            .init_resource::<IsosurfaceBindGroupsCollection>()
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
