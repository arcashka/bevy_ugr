use bevy::{pbr::PbrPlugin, prelude::*};

use bevy_flycam::PlayerPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_isosurface::{Isosurface, IsosurfacePlugin, Polygonization};

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(-100.0, 100.0, -100.0).looking_at(Vec3::ZERO, Vec3::Z),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 20000.0,
            ..default()
        },
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(Cuboid::default())),
        material: materials.add(StandardMaterial {
            base_color: Color::GREEN,
            ..default()
        }),
        transform: Transform::from_xyz(5.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn((
        Isosurface {
            radius: 5.0,
            center: Vec3::new(0.0, 0.0, 0.0),
            fake_mesh_asset: meshes.add(Cuboid::default()).into(),
        },
        Polygonization {
            grid_size: Vec3::new(20.0, 20.0, 20.0),
            grid_origin: Vec3::new(-10.0, -10.0, -10.0),
        },
        materials.add(StandardMaterial {
            base_color: Color::ORANGE_RED,
            unlit: false,
            cull_mode: None,
            ..default()
        }),
        SpatialBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::Visible,
            ..default()
        },
    ));
}

fn main() {
    App::new()
        .add_plugins((
            // scene::ScenePlugin,
            // input::InputPlugin,
            // player::PlayerPlugin,
            // field::FieldPlugin,
            // asset_loader::AssetLoaderPlugin,
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        // uncomment for unthrottled FPS
                        // present_mode: bevy::window::PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(PbrPlugin {
                    prepass_enabled: false,
                    ..default()
                }),
            WorldInspectorPlugin::new(),
            PlayerPlugin,
            IsosurfacePlugin,
        ))
        .add_systems(Startup, setup)
        .register_type::<Isosurface>()
        .run();
}
