use bevy::{
    ecs::entity::EntityHashMap,
    pbr::MeshTransforms,
    prelude::*,
    render::render_resource::{Buffer, ShaderType},
    utils::HashMap,
};

use crate::assets::Isosurface;

pub struct IsosurfaceInstance {
    pub asset_id: AssetId<Isosurface>,
    // TODO: do it as a component instead
    pub fake_mesh_asset: AssetId<Mesh>,
    pub transforms: MeshTransforms,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct IsosurfaceInstances(EntityHashMap<IsosurfaceInstance>);

#[derive(ShaderType, Copy, Clone, Debug, PartialEq, Reflect, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct IsosurfaceIndices {
    pub start: u32,
    pub count: u32,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct IsosurfaceIndicesCollection(HashMap<AssetId<Isosurface>, IsosurfaceIndices>);

pub struct IsosurfaceBuffers {
    pub uniform_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub cells_buffer: Buffer,
    pub atomics_buffer: Buffer,
    pub indices_buffer: Buffer,
    pub indirect_buffer: Buffer,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct IsosurfaceBuffersCollection(HashMap<AssetId<Isosurface>, IsosurfaceBuffers>);
