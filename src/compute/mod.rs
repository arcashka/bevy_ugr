mod node;
mod pipeline;

use bevy::{
    app::{App, Plugin},
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    prelude::*,
    render::{
        render_graph::RenderGraphApp, render_resource::PipelineCache, Render, RenderApp, RenderSet,
    },
    utils::HashMap,
};
use pipeline::{
    check_pipeline_for_readiness, mark_tasks_as_done, prepare_bind_groups, prepare_buffers,
    BuildIndirectBufferBindGroups, CalculateIsosurfaceBindGroups, IndirectBuffersCollection,
    IsosurfaceBuffersCollection, PipelinesReady,
};

use crate::Isosurface;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct CalculateIsosurfaceTasks(HashMap<AssetId<Isosurface>, bool>);

pub struct ComputeIsosurfacePlugin;

impl Plugin for ComputeIsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .add_systems(
                Render,
                (
                    check_pipeline_for_readiness
                        .in_set(RenderSet::Render)
                        .after(PipelineCache::process_pipeline_queue_system),
                    prepare_buffers.in_set(RenderSet::PrepareResources),
                    prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    mark_tasks_as_done.in_set(RenderSet::Cleanup),
                ),
            )
            .init_resource::<CalculateIsosurfaceTasks>()
            .init_resource::<IndirectBuffersCollection>()
            .init_resource::<IsosurfaceBuffersCollection>()
            .init_resource::<CalculateIsosurfaceBindGroups>()
            .init_resource::<BuildIndirectBufferBindGroups>()
            .init_resource::<PipelinesReady>()
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
