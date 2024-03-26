use bevy::{
    ecs::entity::EntityHashMap,
    pbr::{MaterialBindGroupId, MeshTransforms},
    prelude::*,
    render::render_resource::Buffer,
    utils::HashMap,
};

use crate::{assets::IsosurfaceAsset, compute::Indices};

pub struct IsosurfaceInstance {
    pub asset_id: AssetId<IsosurfaceAsset>,
    // TODO: do it as a component instead
    pub fake_mesh_asset: AssetId<Mesh>,
    pub material_bind_group_id: MaterialBindGroupId,
    pub transforms: MeshTransforms,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct IsosurfaceInstances(EntityHashMap<IsosurfaceInstance>);

pub struct IsosurfaceBuffers {
    pub uniform_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub cells_buffer: Buffer,
    pub atomics_buffer: Buffer,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct IsosurfaceBuffersCollection(HashMap<AssetId<IsosurfaceAsset>, IsosurfaceBuffers>);

pub struct PrepareIndirect {
    pub entity: Entity,
    pub asset_id: AssetId<IsosurfaceAsset>,
    pub indices: Indices,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PrepareIndirects(pub Vec<PrepareIndirect>);
