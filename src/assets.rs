use bevy::{
    prelude::*,
    render::render_asset::{RenderAsset, RenderAssetUsages},
};

#[derive(Asset, Clone, Reflect)]
pub struct Isosurface {
    pub grid_size: Vec3,
    pub grid_origin: Vec3,
    // TODO: there is a better way probably...
    //
    // amount of cells in grid is calculated like this
    // x = 8 * grid_density.x
    // y = 8 * grid_density.y
    // z = 8 * grid_density.z
    pub grid_density: UVec3,
    pub asset_usage: RenderAssetUsages,
}

impl Default for Isosurface {
    fn default() -> Self {
        Self {
            grid_size: Vec3::splat(10.0),
            grid_origin: Vec3::ZERO,
            grid_density: UVec3::splat(1),
            asset_usage: RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        }
    }
}

pub struct GpuIsosurface {
    pub grid_size: Vec3,
    pub grid_origin: Vec3,
    pub grid_density: UVec3,
}

impl RenderAsset for Isosurface {
    type PreparedAsset = GpuIsosurface;
    type Param = ();

    fn asset_usage(&self) -> RenderAssetUsages {
        self.asset_usage
    }

    fn prepare_asset(
        self,
        _: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, bevy::render::render_asset::PrepareAssetError<Self>> {
        Ok(GpuIsosurface {
            grid_size: self.grid_size,
            grid_origin: self.grid_origin,
            grid_density: self.grid_density,
        })
    }
}
