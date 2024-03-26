use bevy::{
    asset::AssetId,
    ecs::{entity::Entity, system::lifetimeless::SRes},
    pbr::{MaterialBindGroupId, MeshUniform},
    render::batching::GetBatchData,
};

use crate::{types::IsosurfaceInstances, IsosurfaceAsset};

pub struct IsosurfaceBatcher;

impl GetBatchData for IsosurfaceBatcher {
    type Param = SRes<IsosurfaceInstances>;
    // The material bind group ID and the isosurface asset ID
    type CompareData = (MaterialBindGroupId, AssetId<IsosurfaceAsset>);
    type BufferData = MeshUniform;

    fn get_batch_data(
        isosurface_instances: &bevy::ecs::system::SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let instance = isosurface_instances.get(&entity)?;

        Some((
            MeshUniform::new(&instance.transforms, None),
            Some((instance.material_bind_group_id, instance.asset_id)),
        ))
    }
}
