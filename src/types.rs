use bevy::{
    ecs::entity::EntityHashMap,
    pbr::MeshTransforms,
    prelude::*,
    render::render_resource::{BindGroup, Buffer, ShaderType},
};

pub struct IsosurfaceInstance {
    pub fake_mesh_asset: AssetId<Mesh>,
    pub grid_density: UVec3,
    pub uniforms: IsosurfaceUniforms,
    pub uniform_buffer: Option<Buffer>,
    pub vertex_buffer: Option<Buffer>,
    pub index_buffer: Option<Buffer>,
    pub cell_buffer: Option<Buffer>,
    pub atomics_buffer: Option<Buffer>,
    pub indices_buffer: Option<Buffer>,
    pub indirect_buffer: Option<Buffer>,
    pub compute_bind_group: Option<BindGroup>,
    pub indices: Option<IsosurfaceIndices>,
    pub transforms: MeshTransforms,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct IsosurfaceInstances(EntityHashMap<IsosurfaceInstance>);

#[derive(Component)]
pub struct FakeMesh(pub Handle<Mesh>);

#[derive(ShaderType, Copy, Clone, Debug, PartialEq, Reflect, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct IsosurfaceUniforms {
    pub grid_size: Vec3,
    _padding0: u32,
    pub grid_origin: Vec3,
    _padding1: u32,
    pub sphere_origin: Vec3,
    pub sphere_radius: f32,
}

impl IsosurfaceUniforms {
    pub fn new(
        grid_size: Vec3,
        grid_origin: Vec3,
        sphere_origin: Vec3,
        sphere_radius: f32,
    ) -> Self {
        Self {
            grid_size,
            _padding0: 0,
            grid_origin,
            _padding1: 0,
            sphere_origin,
            sphere_radius,
        }
    }
}

#[derive(ShaderType, Copy, Clone, Debug, PartialEq, Reflect, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct IsosurfaceIndices {
    pub start: u32,
    pub count: u32,
}

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
