use bevy::{
    pbr::{MaterialBindGroupId, MeshTransforms},
    prelude::*,
    render::sync_world::MainEntityHashMap,
};

use crate::assets::IsosurfaceAsset;

pub struct IsosurfaceInstance {
    pub asset_id: AssetId<IsosurfaceAsset>,
    pub material_bind_group_id: MaterialBindGroupId,
    pub transforms: MeshTransforms,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct IsosurfaceInstances(pub MainEntityHashMap<IsosurfaceInstance>);
