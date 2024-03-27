use bevy::{
    ecs::entity::EntityHashMap,
    pbr::{MaterialBindGroupId, MeshTransforms},
    prelude::*,
};

use crate::assets::IsosurfaceAsset;

pub struct IsosurfaceInstance {
    pub asset_id: AssetId<IsosurfaceAsset>,
    pub material_bind_group_id: MaterialBindGroupId,
    pub transforms: MeshTransforms,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct IsosurfaceInstances(EntityHashMap<IsosurfaceInstance>);
