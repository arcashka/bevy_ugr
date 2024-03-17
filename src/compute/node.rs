use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderGraphContext, RenderLabel},
        render_resource::{ComputePassDescriptor, PipelineCache},
        renderer::RenderContext,
    },
};

use super::{CalculateIsosurfaces, IsosurfaceBindGroupsCollection, IsosurfaceComputePipelines};

use crate::assets::IsosurfaceAssetsStorage;

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
        let compute_pipeline = world.resource::<IsosurfaceComputePipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let (
            Some(find_vertices_pipeline),
            Some(connect_vertices_pipeline),
            Some(prepare_indirect_buffer_pipeline),
        ) = (
            pipeline_cache.get_compute_pipeline(compute_pipeline.find_vertices_pipeline),
            pipeline_cache.get_compute_pipeline(compute_pipeline.connect_vertices_pipeline),
            pipeline_cache.get_compute_pipeline(compute_pipeline.prepare_indirect_buffer_pipeline),
        )
        else {
            info!("Pipelines are not ready yet");
            return Ok(());
        };
        let encoder = render_context.command_encoder();
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());

        let assets = world.resource::<IsosurfaceAssetsStorage>();
        let isosurfaces = world.resource::<CalculateIsosurfaces>();
        let bind_groups = world.resource::<IsosurfaceBindGroupsCollection>();

        info!("Node: isosurfaces count: {}", isosurfaces.len());
        for isosurface in isosurfaces.iter() {
            let Some(asset) = assets.get(isosurface) else {
                error!("missing isosurface asset");
                return Ok(());
            };
            let Some(bind_group) = bind_groups.get(isosurface) else {
                error!("missing isosurface compute bind group");
                return Ok(());
            };
            info!("making compute pass for isosurface {}", isosurface);
            let density = asset.grid_density;
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_pipeline(find_vertices_pipeline);
            pass.dispatch_workgroups(density.x, density.y, density.z);
            pass.set_pipeline(connect_vertices_pipeline);
            pass.dispatch_workgroups(density.x, density.y, density.z);
            pass.set_pipeline(prepare_indirect_buffer_pipeline);
            pass.dispatch_workgroups(1, 1, 1);
        }
        Ok(())
    }
}
