use std::hash::Hash;

use bevy::{
    core_pipeline::{
        core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d},
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
        tonemapping::{DebandDither, Tonemapping},
    },
    pbr::{
        alpha_mode_pipeline_key, irradiance_volume::IrradianceVolume,
        screen_space_specular_transmission_pipeline_key, tonemapping_pipeline_key,
        MaterialPipelineKey, MeshFlags, MeshPipelineKey, MeshTransforms, NotShadowReceiver,
        OpaqueRendererMethod, PreviousGlobalTransform, RenderMaterialInstances, RenderMaterials,
        RenderViewLightProbes, ScreenSpaceAmbientOcclusionSettings, ShadowFilteringMethod,
        TransmittedShadowReceiver,
    },
    prelude::*,
    render::{
        camera::TemporalJitter,
        mesh::{InnerMeshVertexBufferLayout, MeshVertexBufferLayout},
        render_phase::{CachedRenderPipelinePhaseItem, DrawFunctions, RenderPhase},
        render_resource::{
            BindGroupEntry, BufferDescriptor, BufferInitDescriptor, BufferUsages, PipelineCache,
            PrimitiveTopology, SpecializedMeshPipelines, VertexAttribute, VertexBufferLayout,
            VertexStepMode,
        },
        renderer::RenderDevice,
        view::{ExtractedView, VisibleEntities},
        Extract,
    },
};

use crate::{
    compute::IsosurfaceComputePipeline,
    draw::{DrawIsosurfaceMaterial, IsosurfaceMaterialPipeline, IsosurfaceMaterialPipelineKey},
    types::{
        DrawIndexedIndirect, FakeMesh, IsosurfaceIndices, IsosurfaceInstance, IsosurfaceInstances,
        IsosurfaceUniforms,
    },
    Isosurface, Polygonization,
};

#[allow(clippy::too_many_arguments)]
pub fn queue_material_isosurfaces<M: Material>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transmissive_draw_functions: Res<DrawFunctions<Transmissive3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<IsosurfaceMaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<IsosurfaceMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_materials: Res<RenderMaterials<M>>,
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&ShadowFilteringMethod>,
        Has<ScreenSpaceAmbientOcclusionSettings>,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
        Option<&Camera3d>,
        Has<TemporalJitter>,
        Option<&Projection>,
        &mut RenderPhase<Opaque3d>,
        &mut RenderPhase<AlphaMask3d>,
        &mut RenderPhase<Transmissive3d>,
        &mut RenderPhase<Transparent3d>,
        (
            Has<RenderViewLightProbes<EnvironmentMapLight>>,
            Has<RenderViewLightProbes<IrradianceVolume>>,
        ),
    )>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (
        view,
        visible_entities,
        tonemapping,
        dither,
        shadow_filter_method,
        ssao,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
        camera_3d,
        temporal_jitter,
        projection,
        mut opaque_phase,
        mut alpha_mask_phase,
        mut transmissive_phase,
        mut transparent_phase,
        (has_environment_maps, _has_irradiance_volumes),
    ) in &mut views
    {
        let draw_opaque_pbr = opaque_draw_functions
            .read()
            .id::<DrawIsosurfaceMaterial<M>>();
        let draw_alpha_mask_pbr = alpha_mask_draw_functions
            .read()
            .id::<DrawIsosurfaceMaterial<M>>();
        let draw_transmissive_pbr = transmissive_draw_functions
            .read()
            .id::<DrawIsosurfaceMaterial<M>>();
        let draw_transparent_pbr = transparent_draw_functions
            .read()
            .id::<DrawIsosurfaceMaterial<M>>();

        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        if temporal_jitter {
            view_key |= MeshPipelineKey::TEMPORAL_JITTER;
        }

        if has_environment_maps {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        if let Some(projection) = projection {
            view_key |= match projection {
                Projection::Perspective(_) => MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
                Projection::Orthographic(_) => MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
            };
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Castano13 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13;
            }
            ShadowFilteringMethod::Jimenez14 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14;
            }
        }

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }
        if ssao {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }
        if let Some(camera_3d) = camera_3d {
            view_key |= screen_space_specular_transmission_pipeline_key(
                camera_3d.screen_space_specular_transmission_quality,
            );
        }
        let rangefinder = view.rangefinder3d();
        for visible_entity in &visible_entities.entities {
            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                continue;
            };
            let Some(isosurface_instance) = isosurface_instances.get_mut(visible_entity) else {
                continue;
            };
            let Some(material) = render_materials.get(material_asset_id) else {
                continue;
            };

            let forward = match material.properties.render_method {
                OpaqueRendererMethod::Forward => true,
                OpaqueRendererMethod::Deferred => false,
                OpaqueRendererMethod::Auto => unreachable!(),
            };

            let mut mesh_key = view_key;

            mesh_key |= MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);

            if material.properties.reads_view_transmission_texture {
                mesh_key |= MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE;
            }

            mesh_key |= alpha_mode_pipeline_key(material.properties.alpha_mode);

            let layout = MeshVertexBufferLayout::new(InnerMeshVertexBufferLayout::new(
                [Mesh::ATTRIBUTE_POSITION.id, Mesh::ATTRIBUTE_NORMAL.id].into(),
                VertexBufferLayout {
                    array_stride: Mesh::ATTRIBUTE_POSITION.format.size()
                        + Mesh::ATTRIBUTE_NORMAL.format.size(),
                    step_mode: VertexStepMode::Vertex,
                    attributes: [
                        VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: Mesh::ATTRIBUTE_POSITION.format,
                        },
                        VertexAttribute {
                            shader_location: 1,
                            offset: 12,
                            format: Mesh::ATTRIBUTE_NORMAL.format,
                        },
                    ]
                    .into(),
                },
            ));
            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &material_pipeline,
                IsosurfaceMaterialPipelineKey {
                    material_pipeline_key: MaterialPipelineKey {
                        mesh_key,
                        bind_group_data: material.key.clone(),
                    },
                },
                &layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            let distance = rangefinder
                .distance_translation(&isosurface_instance.transforms.transform.translation)
                + material.properties.depth_bias;
            match material.properties.alpha_mode {
                AlphaMode::Opaque => {
                    if material.properties.reads_view_transmission_texture {
                        transmissive_phase.add(Transmissive3d {
                            entity: *visible_entity,
                            draw_function: draw_transmissive_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            dynamic_offset: None,
                        });
                    } else if forward {
                        opaque_phase.add(Opaque3d {
                            entity: *visible_entity,
                            draw_function: draw_opaque_pbr,
                            pipeline: pipeline_id,
                            asset_id: isosurface_instance.fake_mesh_asset,
                            batch_range: 0..1,
                            dynamic_offset: None,
                        });
                    }
                }
                AlphaMode::Mask(_) => {
                    if material.properties.reads_view_transmission_texture {
                        transmissive_phase.add(Transmissive3d {
                            entity: *visible_entity,
                            draw_function: draw_transmissive_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            dynamic_offset: None,
                        });
                    } else if forward {
                        alpha_mask_phase.add(AlphaMask3d {
                            entity: *visible_entity,
                            draw_function: draw_alpha_mask_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            dynamic_offset: None,
                        });
                    }
                }
                AlphaMode::Blend
                | AlphaMode::Premultiplied
                | AlphaMode::Add
                | AlphaMode::Multiply => {
                    transparent_phase.add(Transparent3d {
                        entity: *visible_entity,
                        draw_function: draw_transparent_pbr,
                        pipeline: pipeline_id,
                        distance,
                        batch_range: 0..1,
                        dynamic_offset: None,
                    });
                }
            }
        }
    }
}

pub fn prepare_buffers(
    render_device: Res<RenderDevice>,
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
) {
    for (_, isosurface) in isosurface_instances.iter_mut() {
        // vbo
        if isosurface.vertex_buffer.is_none() {
            let vertex_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("isosurface vertex buffer"),
                size: 1024 * 256,
                usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            isosurface.vertex_buffer = Some(vertex_buffer);
        }

        // ibo
        if isosurface.index_buffer.is_none() {
            let index_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("isosurface index buffer"),
                size: 1024 * 256,
                usage: BufferUsages::INDEX | BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            isosurface.index_buffer = Some(index_buffer);
        }

        // cells
        if isosurface.cell_buffer.is_none() {
            let cells_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("isosurface cells buffer"),
                size: 1024 * 256,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            isosurface.cell_buffer = Some(cells_buffer);
        }

        // uniform
        // TODO: write new values instead of recreating this 3... buffers
        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("isosurface uniform buffer"),
            contents: bytemuck::bytes_of(&isosurface.uniforms),
            usage: BufferUsages::UNIFORM,
        });
        isosurface.uniform_buffer = Some(uniform_buffer);

        // atomics
        let atomics_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("isosurface atomics buffer"),
            contents: bytemuck::bytes_of(&[0.0, 0.0]),
            usage: BufferUsages::STORAGE,
        });
        isosurface.atomics_buffer = Some(atomics_buffer);

        let Some(indices) = isosurface.indices.as_ref() else {
            error!("isosurface indices are not set");
            return;
        };
        // indices
        let indices_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("isosurface indices buffer"),
            contents: bytemuck::bytes_of(indices),
            usage: BufferUsages::STORAGE,
        });
        isosurface.indices_buffer = Some(indices_buffer);

        // indirect
        if isosurface.indirect_buffer.is_none() {
            let indirect_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("isosurface indirect buffer"),
                size: std::mem::size_of::<DrawIndexedIndirect>() as u64,
                usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
                mapped_at_creation: false,
            });
            isosurface.indirect_buffer = Some(indirect_buffer);
        }
    }
}

pub fn prepare_bind_groups(
    render_device: Res<RenderDevice>,
    isosurface_compute_pipeline: Res<IsosurfaceComputePipeline>,
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
) {
    for (_, isosurface) in isosurface_instances.iter_mut() {
        if isosurface.compute_bind_group.is_some() {
            continue;
        }
        let Some(uniform_buffer) = isosurface.uniform_buffer.as_ref() else {
            error!("isosurface uniform buffer is None");
            return;
        };
        let Some(vertex_buffer) = isosurface.vertex_buffer.as_ref() else {
            error!("isosurface vertex buffer is None");
            return;
        };
        let Some(index_buffer) = isosurface.index_buffer.as_ref() else {
            error!("isosurface index buffer is None");
            return;
        };
        let Some(cell_buffer) = isosurface.cell_buffer.as_ref() else {
            error!("isosurface cell buffer is None");
            return;
        };
        let Some(atomics_buffer) = isosurface.atomics_buffer.as_ref() else {
            error!("isosurface atomics buffer is None");
            return;
        };
        let Some(indices_buffer) = isosurface.indices_buffer.as_ref() else {
            error!("isosurface indices buffer is None");
            return;
        };
        let Some(indirect_buffer) = isosurface.indirect_buffer.as_ref() else {
            error!("isosurface indirect buffer is None");
            return;
        };

        let bind_group = render_device.create_bind_group(
            None,
            &isosurface_compute_pipeline.compute_bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: cell_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: atomics_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: indices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: indirect_buffer.as_entire_binding(),
                },
            ],
        );
        isosurface.compute_bind_group = Some(bind_group);
    }
}

pub fn extract_isosurfaces(
    mut commands: Commands,
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
    isosurface_query: Extract<
        Query<(
            Entity,
            &Isosurface,
            &Polygonization,
            &ViewVisibility,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            Has<NotShadowReceiver>,
            Has<TransmittedShadowReceiver>,
            &FakeMesh,
        )>,
    >,
) {
    isosurface_instances.clear();
    for (
        entity,
        isosurface,
        polygonization_settings,
        view_visibility,
        transform,
        previous_transform,
        not_shadow_receiver,
        transmitted_receiver,
        fake_mesh,
    ) in isosurface_query.iter()
    {
        if !view_visibility.get() {
            return;
        }
        let transform = transform.affine();
        let previous_transform = previous_transform.map(|t| t.0).unwrap_or(transform);
        let mut flags = if not_shadow_receiver {
            MeshFlags::empty()
        } else {
            MeshFlags::SHADOW_RECEIVER
        };
        if transmitted_receiver {
            flags |= MeshFlags::TRANSMITTED_SHADOW_RECEIVER;
        }
        if transform.matrix3.determinant().is_sign_positive() {
            flags |= MeshFlags::SIGN_DETERMINANT_MODEL_3X3;
        }
        let transforms = MeshTransforms {
            transform: (&transform).into(),
            previous_transform: (&previous_transform).into(),
            flags: flags.bits(),
        };
        commands.get_or_spawn(entity);
        isosurface_instances.insert(
            entity,
            IsosurfaceInstance {
                fake_mesh_asset: fake_mesh.0.clone().into(),
                uniforms: IsosurfaceUniforms::new(
                    polygonization_settings.grid_size,
                    polygonization_settings.grid_origin,
                    isosurface.center,
                    isosurface.radius,
                ),
                vertex_buffer: None,
                index_buffer: None,
                cell_buffer: None,
                uniform_buffer: None,
                atomics_buffer: None,
                indices_buffer: None,
                indirect_buffer: None,
                compute_bind_group: None,
                indices: None,
                transforms,
            },
        );
    }
}

pub fn fill_batch_data<I: CachedRenderPipelinePhaseItem>(
    views: Query<&RenderPhase<I>>,
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
) {
    for (entity, isosurface) in isosurface_instances.iter_mut() {
        views.iter().for_each(|view| {
            for phase in &view.items {
                if phase.entity() == *entity {
                    let range = phase.batch_range().clone();
                    let indices = IsosurfaceIndices::new(range.start, range.end - range.start);
                    isosurface.indices = Some(indices);
                    return;
                }
            }
        });
    }
}

// ugly hack, required because there is some logic in queue material meshes which needs mesh id
pub fn insert_fake_mesh(
    mut commands: Commands,
    mut isosurfaces: Query<Entity, (With<Isosurface>, Without<FakeMesh>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for entity in isosurfaces.iter_mut() {
        commands
            .entity(entity)
            .insert(FakeMesh(meshes.add(Cuboid::default())));
    }
}
