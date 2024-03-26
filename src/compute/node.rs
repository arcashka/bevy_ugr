use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderGraphContext, RenderLabel},
        render_resource::{ComputePassDescriptor, PipelineCache},
        renderer::RenderContext,
    },
};

use super::{
    BuildIndirectBufferBindGroups, CalculateIsosurfaceBindGroups, CalculateIsosurfaceTasks,
    IsosurfaceComputePipelines,
};

use crate::{assets::IsosurfaceAssetsStorage, types::PrepareIndirects};

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

        let assets = world.resource::<IsosurfaceAssetsStorage>();
        let calculate_tasks = world.resource::<CalculateIsosurfaceTasks>();
        let prepare_indirects_tasks = world.resource::<PrepareIndirects>();
        let calculate_bind_groups = world.resource::<CalculateIsosurfaceBindGroups>();
        let build_indirect_buffer_bind_groups = world.resource::<BuildIndirectBufferBindGroups>();

        info!(
            "prepare indirects tasks: {:?}",
            prepare_indirects_tasks.len()
        );

        // let mut index = 0;
        // while index < prepare_indirects_tasks.len() {
        //     let item = &prepare_indirects_tasks[index];
        //     let batch_range = item.batch_range();
        //     if batch_range.is_empty() {
        //         index += 1;
        //     } else {
        //         let draw_function = draw_functions.get_mut(item.draw_function()).unwrap();
        //         draw_function.draw(world, render_pass, view, item);
        //         index += batch_range.len();
        //     }
        // }

        for prepare_indirect_task in prepare_indirects_tasks.iter() {
            let calculate_task = calculate_tasks.get(&prepare_indirect_task.asset_id);
            if calculate_task.is_some_and(|ready| !ready) {
                info!("not ready");
                continue;
            }

            let Some(calculate_bind_group) =
                calculate_bind_groups.get(&prepare_indirect_task.asset_id)
            else {
                error!("missing isosurface compute bind group");
                continue;
            };
            pass.set_bind_group(0, calculate_bind_group, &[]);
            if calculate_task.is_some() {
                let Some(asset) = assets.get(&prepare_indirect_task.asset_id) else {
                    error!("missing isosurface asset");
                    continue;
                };
                let density = asset.grid_density;
                pass.set_pipeline(find_vertices_pipeline);
                pass.dispatch_workgroups(density.x, density.y, density.z);
                pass.set_pipeline(connect_vertices_pipeline);
                pass.dispatch_workgroups(density.x, density.y, density.z);
                info!("CALCULATION DISPATCH");
            }

            let Some(prepare_indirect_bind_group) =
                build_indirect_buffer_bind_groups.get(&prepare_indirect_task.entity)
            else {
                error!("missing isosurface compute bind group");
                continue;
            };
            pass.set_bind_group(1, prepare_indirect_bind_group, &[]);
            pass.set_pipeline(prepare_indirect_buffer_pipeline);
            pass.dispatch_workgroups(1, 1, 1);
            info!("dispatch");
        }
        Ok(())
    }
}
