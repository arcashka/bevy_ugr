use bevy::{
    prelude::*,
    render::render_resource::{BindGroup, ShaderType},
    utils::HashMap,
};

use crate::Isosurface;

// it's used similar to the way PhaseItems are used in bevy drawing pipeline
// they are queued in queue_isosurface_calculations and then cleared in cleanup_calculate_isosurface
#[derive(Default, Deref, DerefMut, Debug)]
pub struct CalculateIsosurface {
    #[deref]
    pub asset_id: AssetId<Isosurface>,
    pub marked_for_deletion: bool,
}

impl CalculateIsosurface {
    pub fn new(asset_id: AssetId<Isosurface>) -> Self {
        Self {
            asset_id,
            marked_for_deletion: false,
        }
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct CalculateIsosurfaces(Vec<CalculateIsosurface>);

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
pub struct IsosurfaceBindGroupsCollection(HashMap<AssetId<Isosurface>, BindGroup>);

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
