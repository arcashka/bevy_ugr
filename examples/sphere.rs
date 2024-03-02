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
        mesh: meshes.add(Plane3d::default().mesh().size(10.0, 10.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::GREEN,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn((
        Isosurface {
            radius: 3.0,
            center: Vec3::new(0.0, 0.0, 0.0),
            fake_mesh_asset: meshes.add(Cuboid::default()).into(),
        },
        Polygonization {
            grid_size: Vec3::new(7.0, 7.0, 7.0),
            grid_origin: Vec3::new(0.0, 0.0, 0.0),
        },
        materials.add(StandardMaterial {
            base_color: Color::ORANGE_RED,
            ..default()
        }),
        SpatialBundle {
            transform: Transform::from_xyz(0.0, 3.0, 0.0),
            visibility: Visibility::Visible,
            ..default()
        },
    ));
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window { ..default() }),
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
