mod node;
mod pipeline;

mod systems;
mod types;

use bevy::{
    app::{App, Plugin},
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    prelude::*,
    render::{render_graph::RenderGraphApp, render_resource::Buffer, Render, RenderApp, RenderSet},
    utils::HashMap,
};

use crate::IsosurfaceAsset;

pub struct IsosurfaceBuffers {
    pub uniform_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub cells_buffer: Buffer,
    pub atomics_buffer: Buffer,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct IsosurfaceBuffersCollection(HashMap<AssetId<IsosurfaceAsset>, IsosurfaceBuffers>);

pub struct IndirectBuffers {
    pub indices_buffer: Buffer,
    pub indirect_buffer: Buffer,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct IndirectBuffersCollection(HashMap<Entity, IndirectBuffers>);

pub struct ComputeIsosurfacePlugin;

impl Plugin for ComputeIsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .add_systems(
                Render,
                (
                    (systems::queue_isosurface_calculations).in_set(RenderSet::Queue),
                    (
                        systems::prepare_calculation_buffers,
                        systems::prepare_indirect_buffers,
                    )
                        .in_set(RenderSet::PrepareResources),
                    (
                        systems::prepare_calculate_isosurface_bind_groups,
                        systems::prepare_generate_indirect_buffer_bind_groups,
                    )
                        .in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .init_resource::<types::CalculateIsosurfaceTasks>()
            .init_resource::<IndirectBuffersCollection>()
            .init_resource::<IsosurfaceBuffersCollection>()
            .init_resource::<types::CalculateIsosurfaceBindGroups>()
            .init_resource::<types::BuildIndirectBufferBindGroups>()
            .init_resource::<types::PrepareIndirects>()
            .add_render_graph_node::<node::IsosurfaceComputeNode>(
                Core3d,
                node::IsosurfaceComputeNodeLabel,
            )
            .add_render_graph_edge(
                Core3d,
                node::IsosurfaceComputeNodeLabel,
                Node3d::StartMainPass,
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<pipeline::IsosurfaceComputePipelines>();
    }
}
