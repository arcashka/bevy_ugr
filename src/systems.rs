use bevy::{
    pbr::{
        MaterialBindGroupId, MeshFlags, MeshTransforms, NotShadowReceiver, PreviousGlobalTransform,
        TransmittedShadowReceiver,
    },
    prelude::*,
    render::Extract,
};

use crate::{
    assets::IsosurfaceAsset,
    types::{IsosurfaceInstance, IsosurfaceInstances},
};

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
                material_bind_group_id: MaterialBindGroupId::default(),
                transforms,
            },
        );
    }
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
