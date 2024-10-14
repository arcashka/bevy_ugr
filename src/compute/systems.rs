use bevy::{
    prelude::*,
    render::{
        render_resource::{BindGroupEntry, BufferDescriptor, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};

use crate::{
    assets::{AssetHandled, IsosurfaceAssetsStorage, NewIsosurfaceAssets},
    compute::types::CalculateIsosurfaceTasks,
    types::IsosurfaceInstances,
};

use super::{
    pipeline::IsosurfaceComputePipelines,
    types::{
        BuildIndirectBufferBindGroups, CalculateIsosurfaceBindGroups, DrawIndexedIndirect,
        IsosurfaceUniforms,
    },
    IndirectBuffers, IndirectBuffersCollection, IsosurfaceBuffers, IsosurfaceBuffersCollection,
};

pub fn queue_isosurface_calculations(
    mut calculate_tasks: ResMut<CalculateIsosurfaceTasks>,
    mut new_assets: ResMut<NewIsosurfaceAssets>,
    isosurface_instances: Res<IsosurfaceInstances>,
) {
    for (_, instance) in isosurface_instances.iter() {
        if let Some(asset_handled) = new_assets.get_mut(&instance.asset_id) {
            info!("adding isosurface to calculate");
            calculate_tasks.insert(instance.asset_id);
            *asset_handled = AssetHandled(true);
        }
    }
}

pub fn prepare_buffers(
    render_device: Res<RenderDevice>,
    assets: Res<IsosurfaceAssetsStorage>,
    tasks: Res<CalculateIsosurfaceTasks>,
    mut calculate_buffers_collection: ResMut<IsosurfaceBuffersCollection>,
    mut indirect_buffers_collection: ResMut<IndirectBuffersCollection>,
) {
    for asset_id in tasks.iter() {
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

        let calculate_buffers = IsosurfaceBuffers {
            vertex_buffer,
            index_buffer,
            cells_buffer,
            uniform_buffer,
            atomics_buffer,
        };

        let indirect_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("Indirect buffer"),
            size: std::mem::size_of::<DrawIndexedIndirect>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        indirect_buffers_collection.insert(*asset_id, IndirectBuffers { indirect_buffer });
        calculate_buffers_collection.insert(*asset_id, calculate_buffers);
    }
}

pub fn prepare_bind_groups(
    render_device: Res<RenderDevice>,
    isosurface_compute_pipeline: Res<IsosurfaceComputePipelines>,
    calculate_buffers: Res<IsosurfaceBuffersCollection>,
    indirect_buffers: Res<IndirectBuffersCollection>,
    tasks: Res<CalculateIsosurfaceTasks>,
    mut calculate_bind_groups: ResMut<CalculateIsosurfaceBindGroups>,
    mut indirect_bind_groups: ResMut<BuildIndirectBufferBindGroups>,
) {
    for asset_id in tasks.iter() {
        let Some(calculate_buffers) = calculate_buffers.get(asset_id) else {
            error!("isosurface buffers not found");
            return;
        };

        let Some(indirect_buffers) = indirect_buffers.get(asset_id) else {
            error!("isosurface buffers not found");
            return;
        };

        let calculate_bind_group = render_device.create_bind_group(
            None,
            &isosurface_compute_pipeline.calculation_bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: calculate_buffers.uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: calculate_buffers.vertex_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: calculate_buffers.index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: calculate_buffers.cells_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: calculate_buffers.atomics_buffer.as_entire_binding(),
                },
            ],
        );
        let indirect_bind_group = render_device.create_bind_group(
            None,
            &isosurface_compute_pipeline.indirect_bind_group_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: indirect_buffers.indirect_buffer.as_entire_binding(),
            }],
        );
        indirect_bind_groups.insert(*asset_id, indirect_bind_group);
        calculate_bind_groups.insert(*asset_id, calculate_bind_group);
    }
}
