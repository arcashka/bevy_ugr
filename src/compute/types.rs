use bevy::{
    prelude::*,
    render::render_resource::{BindGroup, ShaderType},
    utils::HashMap,
};

use crate::IsosurfaceAsset;

// it's used similar to the way PhaseItems are used in bevy drawing pipeline
// they are created in queue_isosurface_calculations and then cleared in cleanup_calculate_isosurface
// ready member is used so we can calculate isosurface only once.
// The idea is:
// * in the system before IsosurfaceComputeNode we make a check if compute pipelines are ready, if they are
//   we mark ready to true
// * in IsosurfaceComputeNode we call dispatch if ready is true
// * in the system after IsosurfaceComputeNode we remove CalculateIsosurface's with ready set to true
//
// This hack is needed because it's not possible to mark/remove anything from within the Node,
// since it has read-only access to the world
#[derive(Default, Deref, DerefMut, Debug)]
pub struct CalculateIsosurface {
    #[deref]
    pub asset_id: AssetId<IsosurfaceAsset>,
    pub ready: bool,
}

impl CalculateIsosurface {
    pub fn new(asset_id: AssetId<IsosurfaceAsset>) -> Self {
        Self {
            asset_id,
            ready: false,
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
pub struct IsosurfaceBindGroupsCollection(HashMap<AssetId<IsosurfaceAsset>, BindGroup>);

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
