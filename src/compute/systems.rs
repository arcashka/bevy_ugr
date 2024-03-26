use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupEntry, BufferDescriptor, BufferInitDescriptor, BufferUsages, PipelineCache,
        },
        renderer::RenderDevice,
    },
};

use crate::{
    assets::{AssetHandled, IsosurfaceAssetsStorage, NewIsosurfaceAssets},
    compute::types::CalculateIsosurfaceTasks,
    types::{
        IsosurfaceBuffers, IsosurfaceBuffersCollection, IsosurfaceInstances, PrepareIndirects,
    },
};

use super::{
    types::{
        CalculateIsosurfaceBindGroups, DrawIndexedIndirect, IndirectBuffers,
        IndirectBuffersCollection, IsosurfaceUniforms,
    },
    BuildIndirectBufferBindGroups, IsosurfaceComputePipelines,
};

pub fn queue_isosurface_calculations(
    mut tasks: ResMut<CalculateIsosurfaceTasks>,
    mut new_assets: ResMut<NewIsosurfaceAssets>,
    isosurface_instances: Res<IsosurfaceInstances>,
) {
    for (_, instance) in isosurface_instances.iter() {
        if let Some(asset_handled) = new_assets.get_mut(&instance.asset_id) {
            info!("adding isosurface to calculate");
            tasks.insert(instance.asset_id, false);
            *asset_handled = AssetHandled(true);
        }
    }
}

pub fn prepare_calculation_buffers(
    render_device: Res<RenderDevice>,
    assets: Res<IsosurfaceAssetsStorage>,
    tasks: Res<CalculateIsosurfaceTasks>,
    mut buffers_collection: ResMut<IsosurfaceBuffersCollection>,
) {
    for (asset_id, _) in tasks.iter() {
        let vertex_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("isosurface vertex buffer"),
            size: 1024 * 256,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let index_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("isosurface index buffer"),
            size: 1024 * 256,
            usage: BufferUsages::INDEX | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let cells_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("isosurface cells buffer"),
            size: 1024 * 256,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // TODO: write new values instead of recreating this 3... buffers
        let Some(asset) = assets.get(asset_id) else {
            error!("isosurface asset not found");
            return;
        };
        let uniforms = IsosurfaceUniforms::new(asset.grid_size, asset.grid_origin);
        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("isosurface uniform buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: BufferUsages::UNIFORM,
        });

        let atomics_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("isosurface atomics buffer"),
            contents: bytemuck::bytes_of(&[0.0, 0.0]),
            usage: BufferUsages::STORAGE,
        });

        let buffers = IsosurfaceBuffers {
            vertex_buffer,
            index_buffer,
            cells_buffer,
            uniform_buffer,
            atomics_buffer,
        };
        buffers_collection.insert(*asset_id, buffers);
    }
}

pub fn prepare_indirect_buffers(
    render_device: Res<RenderDevice>,
    tasks: Res<PrepareIndirects>,
    mut indirect_buffers: ResMut<IndirectBuffersCollection>,
) {
    for task in tasks.iter() {
        let indices_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Indices buffer"),
            contents: bytemuck::bytes_of(&task.indices),
            usage: BufferUsages::STORAGE,
        });

        let indirect_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("Indirect buffer"),
            size: std::mem::size_of::<DrawIndexedIndirect>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
        indirect_buffers.insert(
            task.entity,
            IndirectBuffers {
                indices_buffer,
                indirect_buffer,
            },
        );
    }
}

pub fn prepare_calculate_isosurface_bind_groups(
    render_device: Res<RenderDevice>,
    isosurface_compute_pipeline: Res<IsosurfaceComputePipelines>,
    buffers: Res<IsosurfaceBuffersCollection>,
    tasks: Res<CalculateIsosurfaceTasks>,
    mut bind_groups: ResMut<CalculateIsosurfaceBindGroups>,
) {
    for (asset_id, _) in tasks.iter() {
        let Some(buffers) = buffers.get(asset_id) else {
            error!("isosurface buffers not found");
            return;
        };

        let bind_group = render_device.create_bind_group(
            None,
            &isosurface_compute_pipeline.calculation_bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffers.uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: buffers.vertex_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: buffers.index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: buffers.cells_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: buffers.atomics_buffer.as_entire_binding(),
                },
            ],
        );
        bind_groups.insert(*asset_id, bind_group);
    }
}

pub fn prepare_generate_indirect_buffer_bind_groups(
    render_device: Res<RenderDevice>,
    isosurface_compute_pipeline: Res<IsosurfaceComputePipelines>,
    buffers: Res<IndirectBuffersCollection>,
    tasks: Res<PrepareIndirects>,
    mut bind_groups: ResMut<BuildIndirectBufferBindGroups>,
) {
    for task in tasks.iter() {
        let Some(buffers) = buffers.get(&task.entity) else {
            error!("Indirect buffer not found");
            return;
        };

        let bind_group = render_device.create_bind_group(
            None,
            &isosurface_compute_pipeline.indirect_bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffers.indices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: buffers.indirect_buffer.as_entire_binding(),
                },
            ],
        );
        bind_groups.insert(task.entity, bind_group);
    }
}

// please see comment above CalculateIsosurfaces for explanation of reasoning behind this 2 systems
pub fn cleanup_calculated_isosurface(mut tasks: ResMut<CalculateIsosurfaceTasks>) {
    tasks.retain(|_, done| !(*done));
}

pub fn check_calculate_isosurfaces_for_readiness(
    pipelines: Res<IsosurfaceComputePipelines>,
    pipeline_cache: Res<PipelineCache>,
    mut tasks: ResMut<CalculateIsosurfaceTasks>,
) {
    if let (Some(_), Some(_), Some(_)) = (
        pipeline_cache.get_compute_pipeline(pipelines.find_vertices_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.connect_vertices_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.prepare_indirect_buffer_pipeline),
    ) {
        for (_, ready) in tasks.iter_mut() {
            info!("mark isosurafece as ready");
            *ready = true;
        }
    }
}
