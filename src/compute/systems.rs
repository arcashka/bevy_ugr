use bevy::{
    prelude::*,
    render::{
        render_resource::{BufferDescriptor, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};

use crate::{
    assets::NewRenderAssets,
    types::{IsosurfaceBuffers, IsosurfaceBuffersCollection, IsosurfaceIndicesCollection},
    Isosurface,
};

use super::types::{CalculateIsosurfaces, DrawIndexedIndirect, IsosurfaceUniforms};

pub fn queue_isosurface_calculations(
    mut calculate_isosurfaces: ResMut<CalculateIsosurfaces>,
    new_isosurfaces: Res<NewRenderAssets<Isosurface>>,
) {
    for new_isosurface in new_isosurfaces.iter() {
        calculate_isosurfaces.push(*new_isosurface.0)
    }
}

pub fn prepare_buffers(
    render_device: Res<RenderDevice>,
    isosurface_assets: Res<NewRenderAssets<Isosurface>>,
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
        let Some(isosurface_asset) = isosurface_assets.get(isosurface) else {
            error!("isosurface asset not found");
            return;
        };
        let uniforms =
            IsosurfaceUniforms::new(isosurface_asset.grid_size, isosurface_asset.grid_origin);
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

        let Some(indices) = indices.get(isosurface) else {
            error!("isosurface indices are not set");
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
        buffers_collection.insert(*isosurface, buffers);
    }
}
