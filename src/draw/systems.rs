use std::hash::Hash;

use bevy::{
    core_pipeline::{
        core_3d::{AlphaMask3d, Opaque3d, Opaque3dBinKey, Transmissive3d, Transparent3d},
        oit::OrderIndependentTransparencySettings,
        prepass::{
            DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass,
            OpaqueNoLightmap3dBinKey,
        },
        tonemapping::{DebandDither, Tonemapping},
    },
    pbr::{
        alpha_mode_pipeline_key, irradiance_volume::IrradianceVolume,
        screen_space_specular_transmission_pipeline_key, tonemapping_pipeline_key, DrawMaterial,
        MaterialPipeline, MaterialPipelineKey, MeshPipelineKey, OpaqueRendererMethod,
        PreparedMaterial, RenderMaterialInstances, RenderMeshInstances, RenderViewLightProbes,
        ScreenSpaceAmbientOcclusion, ShadowFilteringMethod,
    },
    prelude::*,
    render::{
        camera::TemporalJitter,
        mesh::{MeshVertexBufferLayout, MeshVertexBufferLayouts},
        render_asset::RenderAssets,
        render_phase::{
            BinnedRenderPhaseType, DrawFunctions, PhaseItemExtraIndex, ViewBinnedRenderPhases,
            ViewSortedRenderPhases,
        },
        render_resource::{
            PipelineCache, SpecializedMeshPipelines, VertexAttribute, VertexBufferLayout,
            VertexStepMode,
        },
        view::{ExtractedView, RenderVisibleEntities},
    },
};

use crate::IsosurfaceHandle;

#[allow(clippy::too_many_arguments)]
pub fn queue_material_isosurfaces<M: Material>(
    (
        opaque_draw_functions,
        alpha_mask_draw_functions,
        transmissive_draw_functions,
        transparent_draw_functions,
    ): (
        Res<DrawFunctions<Opaque3d>>,
        Res<DrawFunctions<AlphaMask3d>>,
        Res<DrawFunctions<Transmissive3d>>,
        Res<DrawFunctions<Transparent3d>>,
    ),
    material_pipeline: Res<MaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<MaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    mesh_instances: Res<RenderMeshInstances>,
    render_materials: Res<RenderAssets<PreparedMaterial<M>>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut alpha_mask_render_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3d>>,
    mut transmissive_render_phases: ResMut<ViewSortedRenderPhases<Transmissive3d>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut vertex_buffer_layouts: ResMut<MeshVertexBufferLayouts>,
    views: Query<(
        Entity,
        &ExtractedView,
        &RenderVisibleEntities,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&ShadowFilteringMethod>,
        Has<ScreenSpaceAmbientOcclusion>,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
        Option<&Camera3d>,
        Has<TemporalJitter>,
        Option<&Projection>,
        (
            Has<RenderViewLightProbes<EnvironmentMapLight>>,
            Has<RenderViewLightProbes<IrradianceVolume>>,
        ),
        Has<OrderIndependentTransparencySettings>,
    )>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (
        view_entity,
        view,
        visible_entities,
        msaa,
        tonemapping,
        dither,
        shadow_filter_method,
        ssao,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
        camera_3d,
        temporal_jitter,
        projection,
        (has_environment_maps, has_irradiance_volumes),
        has_oit,
    ) in &views
    {
        info!("draw queue query found instance");
        let (
            Some(opaque_phase),
            Some(alpha_mask_phase),
            Some(transmissive_phase),
            Some(transparent_phase),
        ) = (
            opaque_render_phases.get_mut(&view_entity),
            alpha_mask_render_phases.get_mut(&view_entity),
            transmissive_render_phases.get_mut(&view_entity),
            transparent_render_phases.get_mut(&view_entity),
        )
        else {
            continue;
        };
        info!("1");

        let draw_opaque_pbr = opaque_draw_functions.read().id::<DrawMaterial<M>>();
        let draw_alpha_mask_pbr = alpha_mask_draw_functions.read().id::<DrawMaterial<M>>();
        let draw_transmissive_pbr = transmissive_draw_functions.read().id::<DrawMaterial<M>>();
        let draw_transparent_pbr = transparent_draw_functions.read().id::<DrawMaterial<M>>();

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

        if has_irradiance_volumes {
            view_key |= MeshPipelineKey::IRRADIANCE_VOLUME;
        }

        if has_oit {
            view_key |= MeshPipelineKey::OIT_ENABLED;
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
            ShadowFilteringMethod::Gaussian => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_GAUSSIAN;
            }
            ShadowFilteringMethod::Temporal => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_TEMPORAL;
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
        for (render_entity, visible_entity) in visible_entities.iter::<With<IsosurfaceHandle>>() {
            info!("looking for entity: {:?}", visible_entity);
            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                info!("3");
                continue;
            };
            let Some(mesh_instance) = mesh_instances.render_mesh_queue_data(*visible_entity) else {
                info!("4");
                continue;
            };
            let Some(material) = render_materials.get(*material_asset_id) else {
                info!("5");
                continue;
            };

            let mut mesh_pipeline_key_bits = material.properties.mesh_pipeline_key_bits;
            mesh_pipeline_key_bits.insert(alpha_mode_pipeline_key(
                material.properties.alpha_mode,
                msaa,
            ));
            let mesh_key = view_key | mesh_pipeline_key_bits;

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

            mesh_instance
                .material_bind_group_id
                .set(material.get_bind_group_id());

            let translation = mesh_instance.translation;
            info!("add isosurface phase item");
            match mesh_key
                .intersection(MeshPipelineKey::BLEND_RESERVED_BITS | MeshPipelineKey::MAY_DISCARD)
            {
                MeshPipelineKey::BLEND_OPAQUE | MeshPipelineKey::BLEND_ALPHA_TO_COVERAGE => {
                    if material.properties.reads_view_transmission_texture {
                        let distance = rangefinder.distance_translation(&translation)
                            + material.properties.depth_bias;
                        transmissive_phase.add(Transmissive3d {
                            entity: (*render_entity, *visible_entity),
                            draw_function: draw_transmissive_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            extra_index: PhaseItemExtraIndex::NONE,
                        });
                    } else if material.properties.render_method == OpaqueRendererMethod::Forward {
                        let bin_key = Opaque3dBinKey {
                            draw_function: draw_opaque_pbr,
                            pipeline: pipeline_id,
                            asset_id: mesh_instance.mesh_asset_id,
                            material_bind_group_id: material.get_bind_group_id().0,
                            lightmap_image: None,
                        };
                        opaque_phase.add(
                            bin_key,
                            (*render_entity, *visible_entity),
                            BinnedRenderPhaseType::NonMesh,
                        );
                    }
                }
                // Alpha mask
                MeshPipelineKey::MAY_DISCARD => {
                    if material.properties.reads_view_transmission_texture {
                        let distance = rangefinder.distance_translation(&translation)
                            + material.properties.depth_bias;
                        transmissive_phase.add(Transmissive3d {
                            entity: (*render_entity, *visible_entity),
                            draw_function: draw_transmissive_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            extra_index: PhaseItemExtraIndex::NONE,
                        });
                    } else if material.properties.render_method == OpaqueRendererMethod::Forward {
                        let bin_key = OpaqueNoLightmap3dBinKey {
                            draw_function: draw_alpha_mask_pbr,
                            pipeline: pipeline_id,
                            asset_id: mesh_instance.mesh_asset_id,
                            material_bind_group_id: material.get_bind_group_id().0,
                        };
                        alpha_mask_phase.add(
                            bin_key,
                            (*render_entity, *visible_entity),
                            BinnedRenderPhaseType::NonMesh,
                        );
                    }
                }
                _ => {
                    let distance = rangefinder.distance_translation(&translation)
                        + material.properties.depth_bias;
                    transparent_phase.add(Transparent3d {
                        entity: (*render_entity, *visible_entity),
                        draw_function: draw_transparent_pbr,
                        pipeline: pipeline_id,
                        distance,
                        batch_range: 0..1,
                        extra_index: PhaseItemExtraIndex::NONE,
                    });
                }
            }
        }
    }
}
