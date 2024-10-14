use bevy::{
    pbr::{
        MaterialBindGroupId, MeshFlags, MeshTransforms, NotShadowReceiver, PreviousGlobalTransform,
        TransmittedShadowReceiver,
    },
    prelude::*,
    render::Extract,
    utils::Parallel,
};

use crate::{
    types::{IsosurfaceInstance, IsosurfaceInstances},
    Isosurface,
};

pub fn extract_isosurface_instances(
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
    mut render_isosurface_instance_queues: Local<Parallel<Vec<(Entity, IsosurfaceInstance)>>>,
    isosurfaces_query: Extract<
        Query<(
            Entity,
            &Isosurface,
            &ViewVisibility,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            Has<NotShadowReceiver>,
            Has<TransmittedShadowReceiver>,
        )>,
    >,
) {
    isosurfaces_query.par_iter().for_each_init(
        || render_isosurface_instance_queues.borrow_local_mut(),
        |queue,
         (
            entity,
            isosurface,
            view_visibility,
            transform,
            previous_transform,
            not_shadow_receiver,
            transmitted_receiver,
        )| {
            if !view_visibility.get() {
                return;
            }

            let world_from_local = transform.affine();
            let mut flags = if not_shadow_receiver {
                MeshFlags::empty()
            } else {
                MeshFlags::SHADOW_RECEIVER
            };
            if transmitted_receiver {
                flags |= MeshFlags::TRANSMITTED_SHADOW_RECEIVER;
            }
            if transform.affine().matrix3.determinant().is_sign_positive() {
                flags |= MeshFlags::SIGN_DETERMINANT_MODEL_3X3;
            }
            queue.push((
                entity,
                IsosurfaceInstance {
                    asset_id: isosurface.id(),
                    material_bind_group_id: MaterialBindGroupId::default(),
                    transforms: MeshTransforms {
                        world_from_local: (&world_from_local).into(),
                        previous_world_from_local: (&previous_transform
                            .map(|t| t.0)
                            .unwrap_or(world_from_local))
                            .into(),
                        flags: flags.bits(),
                    },
                },
            ));
        },
    );

    isosurface_instances.clear();
    for queue in render_isosurface_instance_queues.iter_mut() {
        for (entity, render_isosurface_instance) in queue.drain(..) {
            isosurface_instances.insert_unique_unchecked(entity.into(), render_isosurface_instance);
        }
    }
}
