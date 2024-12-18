use bevy::{
    prelude::*,
    render::{
        mesh::allocator::{ElementClass, ElementLayout, MeshAllocator},
        render_asset::RenderAssets,
        render_resource::{
            binding_types, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntries,
            Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages, CachedComputePipelineId,
            CachedPipelineState, ComputePipelineDescriptor, PipelineCache, ShaderStages,
            ShaderType,
        },
        renderer::{RenderDevice, RenderQueue},
    },
    utils::HashMap,
};

use std::{borrow::Cow, num::NonZeroU64};

use crate::{ComputeIsosurface, Isosurface};

use super::CalculateIsosurfaceTasks;

#[derive(Resource)]
pub struct IsosurfaceComputePipelines {
    pub calculation_bind_group_layout: BindGroupLayout,
    pub indirect_bind_group_layout: BindGroupLayout,

    pub prepare_indirect_buffer_pipeline: CachedComputePipelineId,

    pub find_vertices_pipeline: CachedComputePipelineId,
    pub connect_vertices_pipeline: CachedComputePipelineId,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PipelinesReady(pub bool);

pub struct IsosurfaceBuffers {
    pub uniform_buffer: Buffer,
    pub cells_buffer: Buffer,
    pub atomics_buffer: Buffer,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct IsosurfaceBuffersCollection(HashMap<AssetId<Isosurface>, IsosurfaceBuffers>);

pub struct IndirectBuffers {
    pub indirect_buffer: Buffer,
}

#[derive(ShaderType, Copy, Clone, Debug, PartialEq, Reflect, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct IsosurfaceUniforms {
    pub grid_size: Vec3,
    _padding0: u32,
    pub grid_origin: Vec3,
    _padding1: u32,
}

impl IsosurfaceUniforms {
    pub fn new(grid_size: Vec3, grid_origin: Vec3) -> Self {
        Self {
            grid_size,
            _padding0: 0,
            grid_origin,
            _padding1: 0,
        }
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct CalculateIsosurfaceBindGroups(HashMap<AssetId<Isosurface>, BindGroup>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct BuildIndirectBufferBindGroups(HashMap<AssetId<Isosurface>, BindGroup>);

// used only to get it's sizeof
#[derive(ShaderType)]
#[repr(C)]
pub struct DrawIndexedIndirect {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    vertex_offset: i32,
    first_instance: u32,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct IndirectBuffersCollection(HashMap<AssetId<Isosurface>, IndirectBuffers>);

impl FromWorld for IsosurfaceComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let calculation_bind_group_layout = render_device.create_bind_group_layout(
            "isosurface compute bind group layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // Uniforms
                    binding_types::uniform_buffer::<IsosurfaceUniforms>(false),
                    // VBO
                    binding_types::storage_buffer_sized(false, NonZeroU64::new(1024)),
                    // IBO
                    binding_types::storage_buffer_sized(false, NonZeroU64::new(1024)),
                    // Cells, Intermediate buffer
                    binding_types::storage_buffer_sized(false, NonZeroU64::new(1024)),
                    // Atomics
                    binding_types::storage_buffer_sized(false, None),
                ),
            ),
        );

        let indirect_bind_group_layout = render_device.create_bind_group_layout(
            "isosurface compute bind group layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // indirect
                    binding_types::storage_buffer::<DrawIndexedIndirect>(false),
                ),
            ),
        );

        let shader = world
            .resource::<AssetServer>()
            .load("isosurface_compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let find_vertices_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("isosurface find_vertices pipeline".into()),
                layout: vec![calculation_bind_group_layout.clone()],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: Cow::from("find_vertices"),
            });

        let connect_vertices_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("isosurface connect_vertices pipeline".into()),
                layout: vec![calculation_bind_group_layout.clone()],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: Cow::from("connect_vertices"),
            });

        let prepare_indirect_buffer_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("isosurface prepare_indirect_buffer pipeline".into()),
                layout: vec![
                    calculation_bind_group_layout.clone(),
                    indirect_bind_group_layout.clone(),
                ],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: Cow::from("prepare_indirect_buffer"),
            });

        info!("pipelines are queued");
        IsosurfaceComputePipelines {
            calculation_bind_group_layout,
            indirect_bind_group_layout,
            find_vertices_pipeline,
            connect_vertices_pipeline,
            prepare_indirect_buffer_pipeline,
        }
    }
}

pub fn prepare_buffers(
    render_device: Res<RenderDevice>,
    assets: Res<RenderAssets<ComputeIsosurface>>,
    tasks: Res<CalculateIsosurfaceTasks>,
    mut calculate_buffers_collection: ResMut<IsosurfaceBuffersCollection>,
    mut indirect_buffers_collection: ResMut<IndirectBuffersCollection>,
) {
    for (asset_id, _) in tasks.iter() {
        let cells_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("isosurface cells buffer"),
            size: 1024 * 256,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // TODO: write new values instead of recreating this 3... buffers
        let Some(asset) = assets.get(*asset_id) else {
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

#[allow(clippy::too_many_arguments)]
pub fn prepare_bind_groups(
    render_device: Res<RenderDevice>,
    isosurface_compute_pipeline: Res<IsosurfaceComputePipelines>,
    calculate_buffers: Res<IsosurfaceBuffersCollection>,
    indirect_buffers: Res<IndirectBuffersCollection>,
    tasks: Res<CalculateIsosurfaceTasks>,
    mesh_allocator: Res<MeshAllocator>,
    mut calculate_bind_groups: ResMut<CalculateIsosurfaceBindGroups>,
    mut indirect_bind_groups: ResMut<BuildIndirectBufferBindGroups>,
) {
    for (asset_id, mesh_id) in tasks.iter() {
        let Some(calculate_buffers) = calculate_buffers.get(asset_id) else {
            info!("isosurface buffers not found");
            return;
        };

        let Some(indirect_buffers) = indirect_buffers.get(asset_id) else {
            info!("isosurface buffers not found");
            return;
        };

        let (Some(vertex_slice), Some(index_slice)) = (
            mesh_allocator.mesh_vertex_slice(mesh_id),
            mesh_allocator.mesh_index_slice(mesh_id),
        ) else {
            info!("no buffers");
            continue;
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
                    resource: vertex_slice.buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: index_slice.buffer.as_entire_binding(),
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

pub fn check_pipeline_for_readiness(
    pipelines: Res<IsosurfaceComputePipelines>,
    pipeline_cache: Res<PipelineCache>,
    mut is_ready: ResMut<PipelinesReady>,
) {
    if let (CachedPipelineState::Ok(_), CachedPipelineState::Ok(_), CachedPipelineState::Ok(_)) = (
        pipeline_cache.get_compute_pipeline_state(pipelines.find_vertices_pipeline),
        pipeline_cache.get_compute_pipeline_state(pipelines.connect_vertices_pipeline),
        pipeline_cache.get_compute_pipeline_state(pipelines.prepare_indirect_buffer_pipeline),
    ) {
        is_ready.0 = true;
    };
}

pub fn allocate_buffers(
    mut mesh_allocator: ResMut<MeshAllocator>,
    tasks: Res<CalculateIsosurfaceTasks>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for (_, mesh_id) in tasks.iter() {
        if let (Some(_), Some(_)) = (
            mesh_allocator.mesh_vertex_slice(mesh_id),
            mesh_allocator.mesh_index_slice(mesh_id),
        ) {
            continue;
        };

        let vertex_element_layout = ElementLayout::new(
            ElementClass::Vertex,
            Mesh::ATTRIBUTE_POSITION.format.size() + Mesh::ATTRIBUTE_NORMAL.format.size(),
        );
        mesh_allocator.allocate_large(mesh_id, vertex_element_layout);

        let index_element_layout =
            ElementLayout::new(ElementClass::Index, std::mem::size_of::<u32>() as u64);
        mesh_allocator.allocate_large(mesh_id, index_element_layout);

        let Some(vertex_slab_id) = mesh_allocator.mesh_id_to_vertex_slab.get(mesh_id).copied()
        else {
            unreachable!();
        };
        let Some(index_slab_id) = mesh_allocator.mesh_id_to_index_slab.get(mesh_id).copied() else {
            unreachable!();
        };
        mesh_allocator.copy_element_data(
            mesh_id,
            1024 * 256,
            |_| {},
            BufferUsages::VERTEX | BufferUsages::STORAGE,
            vertex_slab_id,
            &render_device,
            &render_queue,
        );
        mesh_allocator.copy_element_data(
            mesh_id,
            1024 * 256,
            |_| {},
            BufferUsages::INDEX | BufferUsages::STORAGE,
            index_slab_id,
            &render_device,
            &render_queue,
        );
        info!("should be done")
    }
}
