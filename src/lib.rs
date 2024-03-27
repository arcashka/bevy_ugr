mod assets;
mod compute;
mod draw;
mod systems;
mod types;

use bevy::{prelude::*, render::RenderApp};

pub use assets::IsosurfaceAsset;

#[derive(Default)]
pub struct IsosurfacePlugin;

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(assets::IsosurfaceAssetsPlugin)
            .add_plugins(draw::DrawIsosurfacePlugin)
            .add_plugins(compute::ComputeIsosurfacePlugin);

        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, systems::extract_isosurfaces)
            .init_resource::<types::IsosurfaceInstances>();
    }
}
