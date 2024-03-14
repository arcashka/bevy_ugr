mod assets;
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
        render_asset::RenderAssetPlugin, render_graph::RenderGraphApp,
        render_phase::AddRenderCommand, Render, RenderApp, RenderSet,
    },
};

use types::{IsosurfaceBindGroups, IsosurfaceInstances};

pub use assets::Isosurface;

#[derive(Default)]
pub struct IsosurfacePlugin;

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, systems::insert_fake_mesh)
            .add_plugins(RenderAssetPlugin::<Isosurface>::default())
            .register_type::<Isosurface>()
            .init_asset::<Isosurface>()
            .register_asset_reflect::<Isosurface>();

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
                    systems::prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .add_render_command::<Transmissive3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Transparent3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Opaque3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<AlphaMask3d, draw::DrawIsosurfaceMaterial<StandardMaterial>>()
            .init_resource::<IsosurfaceInstances>()
            .init_resource::<IsosurfaceBindGroups>()
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
