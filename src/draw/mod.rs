mod commands;
mod systems;
mod types;

use bevy::{
    app::{App, Plugin},
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d},
    pbr::StandardMaterial,
    prelude::*,
    render::{render_phase::AddRenderCommand, Render, RenderApp, RenderSet},
};

pub struct DrawIsosurfacePlugin;

impl Plugin for DrawIsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .add_systems(
                Render,
                (
                    systems::queue_material_isosurfaces::<StandardMaterial>
                        .in_set(RenderSet::Queue),
                ),
            )
            .init_resource::<types::DrawBindGroupLayout>()
            .add_render_command::<Transmissive3d, commands::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Transparent3d, commands::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<Opaque3d, commands::DrawIsosurfaceMaterial<StandardMaterial>>()
            .add_render_command::<AlphaMask3d, commands::DrawIsosurfaceMaterial<StandardMaterial>>();
    }
}
