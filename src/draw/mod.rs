// mod commands;
mod systems;
// mod types;

use bevy::{
    app::{App, Plugin},
    pbr::StandardMaterial,
    prelude::IntoSystemConfigs,
    render::{Render, RenderApp, RenderSet},
};

pub struct DrawIsosurfacePlugin;

impl Plugin for DrawIsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).add_systems(
            Render,
            systems::queue_material_isosurfaces::<StandardMaterial>.in_set(RenderSet::Queue),
        );
    }
}
