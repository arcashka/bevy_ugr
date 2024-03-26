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
        MaterialBindGroupId, MaterialPipeline, MaterialPipelineKey, MeshFlags, MeshPipeline,
        MeshPipelineKey, MeshTransforms, MeshUniform, NotShadowReceiver, OpaqueRendererMethod,
        PreviousGlobalTransform, RenderMaterialInstances, RenderMaterials, RenderViewLightProbes,
        ScreenSpaceAmbientOcclusionSettings, ShadowFilteringMethod, TransmittedShadowReceiver,
    },
    prelude::*,
    render::{
        camera::TemporalJitter,
        mesh::{MeshVertexBufferLayout, MeshVertexBufferLayouts},
        render_phase::{DrawFunctions, RenderPhase},
        render_resource::{
            GpuArrayBuffer, PipelineCache, PrimitiveTopology, SpecializedMeshPipelines,
            VertexAttribute, VertexBufferLayout, VertexStepMode,
        },
        renderer::RenderDevice,
        view::{ExtractedView, VisibleEntities},
        Extract,
    },
};

use crate::{
    assets::IsosurfaceAsset,
    draw::{DrawBindGroups, DrawIsosurfaceMaterial, FakeMesh},
    types::{IsosurfaceInstance, IsosurfaceInstances},
};

#[allow(clippy::too_many_arguments)]
pub fn queue_material_isosurfaces<M: Material>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transmissive_draw_functions: Res<DrawFunctions<Transmissive3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<MaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<MaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_materials: Res<RenderMaterials<M>>,
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    mut vertex_buffer_layouts: ResMut<MeshVertexBufferLayouts>,
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
    info!("queue_material_isosurfaces");
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

            let layout = MeshVertexBufferLayout::new(
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
            );
            let layout_ref = vertex_buffer_layouts.insert(layout);
            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &material_pipeline,
                MaterialPipelineKey {
                    mesh_key,
                    bind_group_data: material.key.clone(),
                },
                &layout_ref,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            isosurface_instance.material_bind_group_id = material.get_bind_group_id();

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
                            asset_id: isosurface_instance.fake_mesh_asset,
                            entity: *visible_entity,
                            draw_function: draw_alpha_mask_pbr,
                            pipeline: pipeline_id,
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

pub fn extract_isosurfaces(
    mut commands: Commands,
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
    isosurface_query: Extract<
        Query<(
            Entity,
            &Handle<IsosurfaceAsset>,
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
                asset_id: isosurface.id(),
                fake_mesh_asset: fake_mesh.0.id(),
                material_bind_group_id: MaterialBindGroupId::default(),
                transforms,
            },
        );
    }
}

// ugly hack, required because there is some logic in queue material meshes which needs mesh id
pub fn insert_fake_mesh(
    mut commands: Commands,
    mut isosurfaces: Query<Entity, (With<Handle<IsosurfaceAsset>>, Without<FakeMesh>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for entity in isosurfaces.iter_mut() {
        commands
            .entity(entity)
            .insert(FakeMesh(meshes.add(Cuboid::default())));
    }
}

// I think it can be done only once?
// then why is it per frame in bevy itself?
// TODO: figure out
pub fn prepare_model_bind_group_layout(
    mut groups: ResMut<DrawBindGroups>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
    mesh_uniforms: Res<GpuArrayBuffer<MeshUniform>>,
) {
    groups.reset();
    let layouts = &mesh_pipeline.mesh_layouts;
    let Some(model) = mesh_uniforms.binding() else {
        return;
    };
    groups.model_only = Some(layouts.model_only(&render_device, &model));
}

// fn fill_tasks<'a, T: PhaseItem>(
//     phase_items: &'a Vec<T>,
//     isosurfaces: &'a EntityHashMap<IsosurfaceInstance>,
//     tasks: &'a mut Vec<PrepareIndirect>,
// ) {
//     let mut index = 0;
//     while index < phase_items.len() {
//         let item = &phase_items[index];
//         let batch_range = item.batch_range();
//         if batch_range.is_empty() {
//             index += 1;
//         } else {
//             if let Some(isosurface) = isosurfaces.get(&item.entity()) {
//                 tasks.push(PrepareIndirect {
//                     entity: item.entity(),
//                     asset_id: isosurface.asset_id,
//                     indices: Indices {
//                         instance_count: range.end - range.start,
//                         first_instance: range.start,
//                     },
//                 });
//             };
//         }
//     }
// }
//
// pub fn queue_prepare_indirects(
//     mut tasks: ResMut<PrepareIndirects>,
//     isosurfaces: Res<IsosurfaceInstances>,
//     views: Query<
//         (
//             &RenderPhase<Opaque3d>,
//             &RenderPhase<AlphaMask3d>,
//             &RenderPhase<Transmissive3d>,
//             &RenderPhase<Transparent3d>,
//         ),
//         With<ExtractedView>,
//     >,
// ) {
//     let to_indices = |range: &Range<u32>| Indices {
//         instance_count: range.end - range.start,
//         first_instance: range.start,
//     };
//     for (opaque, alpha_mask, transmissive, transparent) in views.iter() {}
//     for task in tasks.iter_mut() {
//         for (opaque, alpha_mask, transmissive, transparent) in views.iter() {
//             if let Some(batch_range) = find_batch_range(entity, &opaque.items) {
//                 task.indices = to_indices(batch_range);
//                 break;
//             }
//             if let Some(batch_range) = find_batch_range(entity, &alpha_mask.items) {
//                 task.indices = to_indices(batch_range);
//                 break;
//             }
//             if let Some(batch_range) = find_batch_range(entity, &transmissive.items) {
//                 task.indices = to_indices(batch_range);
//                 break;
//             }
//             if let Some(batch_range) = find_batch_range(entity, &transparent.items) {
//                 task.indices = to_indices(batch_range);
//                 break;
//             }
//         }
//         info!("batch range: {:?}", task.indices);
//     }
// }
