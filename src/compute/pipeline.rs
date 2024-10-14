use bevy::{
    prelude::*,
    render::{
        render_resource::{
            binding_types, BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId,
            ComputePipelineDescriptor, PipelineCache, ShaderStages,
        },
        renderer::RenderDevice,
    },
};

use std::{borrow::Cow, num::NonZeroU64};

use super::types::{DrawIndexedIndirect, Indices, IsosurfaceUniforms};

#[derive(Resource)]
pub struct IsosurfaceComputePipelines {
    pub calculation_bind_group_layout: BindGroupLayout,
    pub indirect_bind_group_layout: BindGroupLayout,

    pub prepare_indirect_buffer_pipeline: CachedComputePipelineId,

    pub find_vertices_pipeline: CachedComputePipelineId,
    pub connect_vertices_pipeline: CachedComputePipelineId,
}

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
                    // indices
                    binding_types::storage_buffer::<Indices>(false),
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
