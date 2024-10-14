use bevy::{pbr::PbrPlugin, prelude::*};

use bevy_ugr::{Isosurface, IsosurfaceAsset, IsosurfacePlugin};

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut isosurfaces: ResMut<Assets<IsosurfaceAsset>>,
) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 20000.0,
            ..default()
        },
        SpatialBundle::from_transform(Transform::from_xyz(0.0, 0.0, 0.0)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::GREEN.into(),
            ..default()
        })),
        SpatialBundle::from_transform(Transform::from_xyz(0.0, 0.0, 0.0)),
    ));

    commands.spawn((
        Isosurface(isosurfaces.add(IsosurfaceAsset {
            grid_size: Vec3::new(7.0, 7.0, 7.0),
            grid_origin: Vec3::new(0.0, 0.0, 0.0),
            grid_density: UVec3::new(1, 1, 1),
            ..default()
        })),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::RED.into(),
            ..default()
        })),
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
            IsosurfacePlugin,
        ))
        .add_systems(Startup, setup)
        .register_type::<IsosurfaceAsset>()
        .run();
}
