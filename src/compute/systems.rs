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
    compute::types::CalculateIsosurface,
    types::{
        IsosurfaceBuffers, IsosurfaceBuffersCollection, IsosurfaceIndicesCollection,
        IsosurfaceInstances,
    },
};

use super::{
    types::{
        CalculateIsosurfaces, DrawIndexedIndirect, IsosurfaceBindGroupsCollection,
        IsosurfaceUniforms,
    },
    IsosurfaceComputePipelines,
};

pub fn queue_isosurface_calculations(
    mut calculate_isosurfaces: ResMut<CalculateIsosurfaces>,
    mut new_assets: ResMut<NewIsosurfaceAssets>,
    isosurfaces: Res<IsosurfaceInstances>,
) {
    for (_, isosurface) in isosurfaces.iter() {
        if let Some(asset_handled) = new_assets.get_mut(&isosurface.asset_id) {
            info!("adding isosurface to calculate");
            calculate_isosurfaces.push(CalculateIsosurface::new(isosurface.asset_id));
            *asset_handled = AssetHandled(true);
        }
    }
}

pub fn prepare_buffers(
    render_device: Res<RenderDevice>,
    assets: Res<IsosurfaceAssetsStorage>,
    calculate_isosurfaces: Res<CalculateIsosurfaces>,
    indices: Res<IsosurfaceIndicesCollection>,
    mut buffers_collection: ResMut<IsosurfaceBuffersCollection>,
) {
    for isosurface in calculate_isosurfaces.iter() {
        // vbo
        let vertex_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("isosurface vertex buffer"),
            size: 1024 * 256,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // ibo
        let index_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("isosurface index buffer"),
            size: 1024 * 256,
            usage: BufferUsages::INDEX | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // cells
        let cells_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("isosurface cells buffer"),
            size: 1024 * 256,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // uniform
        // TODO: write new values instead of recreating this 3... buffers
        let Some(asset) = assets.get(&isosurface.asset_id) else {
            error!("isosurface asset not found");
            return;
        };
        let uniforms = IsosurfaceUniforms::new(asset.grid_size, asset.grid_origin);
        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("isosurface uniform buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: BufferUsages::UNIFORM,
        });

        // atomics
        let atomics_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("isosurface atomics buffer"),
            contents: bytemuck::bytes_of(&[0.0, 0.0]),
            usage: BufferUsages::STORAGE,
        });

        let Some(indices) = indices.get(&isosurface.asset_id) else {
            error!("isosurface indices are not set for asset: {:?}", isosurface);
            return;
        };
        // indices
        let indices_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("isosurface indices buffer"),
            contents: bytemuck::bytes_of(indices),
            usage: BufferUsages::STORAGE,
        });

        // indirect
        let indirect_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("isosurface indirect buffer"),
            size: std::mem::size_of::<DrawIndexedIndirect>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
        let buffers = IsosurfaceBuffers {
            vertex_buffer,
            index_buffer,
            cells_buffer,
            uniform_buffer,
            atomics_buffer,
            indices_buffer,
            indirect_buffer,
        };
        buffers_collection.insert(isosurface.asset_id, buffers);
    }
}

pub fn prepare_bind_groups(
    render_device: Res<RenderDevice>,
    isosurface_compute_pipeline: Res<IsosurfaceComputePipelines>,
    buffers: Res<IsosurfaceBuffersCollection>,
    isosurfaces: Res<CalculateIsosurfaces>,
    mut bind_groups: ResMut<IsosurfaceBindGroupsCollection>,
) {
    for isosurface in isosurfaces.iter() {
        let Some(buffers) = buffers.get(&isosurface.asset_id) else {
            error!("isosurface buffers not found");
            return;
        };

        let bind_group = render_device.create_bind_group(
            None,
            &isosurface_compute_pipeline.compute_bind_group_layout,
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
                BindGroupEntry {
                    binding: 5,
                    resource: buffers.indices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: buffers.indirect_buffer.as_entire_binding(),
                },
            ],
        );
        bind_groups.insert(isosurface.asset_id, bind_group);
    }
}

// DIRTY HACK WARNING
// The idea is to remove CalculateIsosurfaces when we calculated them
// but we can't do that in the node which calculates them, since we don't have a mutable
// access to the world there.
// So instead we check for node code preconditions here and assume, if they are met here
// then node code also was (or will be next frame) successfully executed
pub fn cleanup_calculate_isosurface(
    pipelines: Res<IsosurfaceComputePipelines>,
    pipeline_cache: Res<PipelineCache>,
    mut isosurfaces: ResMut<CalculateIsosurfaces>,
) {
    if let (Some(_), Some(_), Some(_)) = (
        pipeline_cache.get_compute_pipeline(pipelines.find_vertices_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.connect_vertices_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.prepare_indirect_buffer_pipeline),
    ) {
        isosurfaces.retain(|isosurface| !isosurface.marked_for_deletion);
        for isosurface in isosurfaces.iter_mut() {
            isosurface.marked_for_deletion = true;
        }
    }
}
