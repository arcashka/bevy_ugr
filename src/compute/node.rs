use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, RenderGraphContext, RenderLabel},
        render_resource::{ComputePassDescriptor, PipelineCache},
        renderer::RenderContext,
    },
};

use crate::ComputeIsosurface;

use super::{
    pipeline::IsosurfaceComputePipelines, BuildIndirectBufferBindGroups,
    CalculateIsosurfaceBindGroups, CalculateIsosurfaceTasks,
};

#[derive(Default)]
pub struct IsosurfaceComputeNode;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct IsosurfaceComputeNodeLabel;

impl render_graph::Node for IsosurfaceComputeNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let compute_pipelines = world.resource::<IsosurfaceComputePipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let (
            Some(find_vertices_pipeline),
            Some(connect_vertices_pipeline),
            Some(prepare_indirect_buffer_pipeline),
        ) = (
            pipeline_cache.get_compute_pipeline(compute_pipelines.find_vertices_pipeline),
            pipeline_cache.get_compute_pipeline(compute_pipelines.connect_vertices_pipeline),
            pipeline_cache.get_compute_pipeline(compute_pipelines.prepare_indirect_buffer_pipeline),
        )
        else {
            return Ok(());
        };

        let encoder = render_context.command_encoder();
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());

        let assets = world.resource::<RenderAssets<ComputeIsosurface>>();
        let calculate_tasks = world.resource::<CalculateIsosurfaceTasks>();
        let calculate_bind_groups = world.resource::<CalculateIsosurfaceBindGroups>();
        let build_indirect_buffer_bind_groups = world.resource::<BuildIndirectBufferBindGroups>();

        for (asset_id, _) in calculate_tasks.iter() {
            let Some(calculate_bind_group) = calculate_bind_groups.get(asset_id) else {
                error!("missing isosurface compute bind group");
                continue;
            };
            pass.set_bind_group(0, calculate_bind_group, &[]);
            let Some(isosurface) = assets.get(*asset_id) else {
                error!("missing isosurface asset");
                continue;
            };
            let density = isosurface.grid_density;
            pass.set_pipeline(find_vertices_pipeline);
            pass.dispatch_workgroups(density.x, density.y, density.z);
            pass.set_pipeline(connect_vertices_pipeline);
            pass.dispatch_workgroups(density.x, density.y, density.z);

            let Some(prepare_indirect_bind_group) = build_indirect_buffer_bind_groups.get(asset_id)
            else {
                error!("missing isosurface compute bind group");
                continue;
            };
            pass.set_bind_group(1, prepare_indirect_bind_group, &[]);
            pass.set_pipeline(prepare_indirect_buffer_pipeline);
            pass.dispatch_workgroups(1, 1, 1);
            info!("isosurface compute pass done");
        }
        Ok(())
    }
}
