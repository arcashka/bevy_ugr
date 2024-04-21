use bevy::{
    asset::AssetId,
    ecs::{
        entity::Entity,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    pbr::{MaterialBindGroupId, MeshInputUniform, MeshUniform, RenderMeshInstances},
    render::batching::{GetBatchData, GetFullBatchData},
};
use nonmax::NonMaxU32;

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

impl GetFullBatchData for IsosurfaceBatcher {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        isosurface_instances: &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref isosurface_instances) = **isosurface_instances
        else {
            error!(
                "`get_index_and_compare_data` should never be called in CPU mesh uniform building \
                mode"
            );
            return None;
        };

        let mesh_instance = mesh_instances.get(&entity)?;
        let maybe_lightmap = lightmaps.render_lightmaps.get(&entity);

        Some((
            mesh_instance.current_uniform_index,
            mesh_instance.should_batch().then_some((
                mesh_instance.material_bind_group_id.get(),
                mesh_instance.mesh_asset_id,
                maybe_lightmap.map(|lightmap| lightmap.image),
            )),
        ))
    }

    fn get_binned_batch_data(
        (mesh_instances, lightmaps): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&entity)?;
        let maybe_lightmap = lightmaps.render_lightmaps.get(&entity);

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            maybe_lightmap.map(|lightmap| lightmap.uv_rect),
        ))
    }

    fn get_binned_index(
        (mesh_instances, _): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<NonMaxU32> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_index` should never be called in CPU mesh uniform \
                building mode"
            );
            return None;
        };

        mesh_instances
            .get(&entity)
            .map(|entity| entity.current_uniform_index)
    }
}
