use bevy::{pbr::PbrPlugin, prelude::*, render::view::NoFrustumCulling};

use bevy_ugr::{Isosurface, IsosurfaceHandle, IsosurfacePlugin};

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut isosurfaces: ResMut<Assets<Isosurface>>,
) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 20000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Visible,
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mesh_asset = meshes.add(Plane3d::default().mesh().size(10.0, 10.0));
    info!("mesh asset untyped: {}", mesh_asset.id().untyped());
    commands.spawn((
        Mesh3d(mesh_asset),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::GREEN.into(),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Visible,
    ));

    let isosurface_asset = isosurfaces.add(Isosurface {
        grid_size: Vec3::new(7.0, 7.0, 7.0),
        grid_origin: Vec3::new(0.0, 0.0, 0.0),
        grid_density: UVec3::new(1, 1, 1),
        ..default()
    });
    info!(
        "isosurface asset untyped: {}",
        isosurface_asset.id().untyped()
    );

    commands.spawn((
        IsosurfaceHandle(isosurface_asset),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::RED.into(),
            ..default()
        })),
        Transform::from_xyz(0.0, 3.0, 0.0),
        Visibility::Visible,
        NoFrustumCulling,
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
                    use_gpu_instance_buffer_builder: false,
                    prepass_enabled: false,
                    ..default()
                }),
            IsosurfacePlugin,
        ))
        .add_systems(Startup, setup)
        .register_type::<Isosurface>()
        .run();
}
