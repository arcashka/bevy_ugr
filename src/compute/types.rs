use bevy::{
    prelude::*,
    render::render_resource::{BindGroup, ShaderType},
    utils::{HashMap, HashSet},
};

use crate::IsosurfaceAsset;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct CalculateIsosurfaceTasks(HashSet<AssetId<IsosurfaceAsset>>);

#[derive(ShaderType, Copy, Clone, Debug, PartialEq, Reflect, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct IsosurfaceUniforms {
    pub grid_size: Vec3,
    _padding0: u32,
    pub grid_origin: Vec3,
    _padding1: u32,
}

impl IsosurfaceUniforms {
    pub fn new(grid_size: Vec3, grid_origin: Vec3) -> Self {
        Self {
            grid_size,
            _padding0: 0,
            grid_origin,
            _padding1: 0,
        }
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct CalculateIsosurfaceBindGroups(HashMap<AssetId<IsosurfaceAsset>, BindGroup>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct BuildIndirectBufferBindGroups(HashMap<Entity, BindGroup>);

// used only to get it's sizeof
#[derive(ShaderType)]
#[repr(C)]
pub struct DrawIndexedIndirect {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    vertex_offset: i32,
    first_instance: u32,
}

#[derive(ShaderType, Copy, Clone, Reflect, bytemuck::Pod, bytemuck::Zeroable, Default, Debug)]
#[repr(C)]
pub struct Indices {
    pub first_instance: u32,
    pub instance_count: u32,
}

pub struct PrepareIndirect {
    pub entity: Entity,
    pub asset_id: AssetId<IsosurfaceAsset>,
    pub indices: Indices,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PrepareIndirects(pub Vec<PrepareIndirect>);
