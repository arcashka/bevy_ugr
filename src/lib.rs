mod assets;
mod compute;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        mesh::PrimitiveTopology,
        view::{check_visibility, VisibilitySystems},
        Extract, RenderApp,
    },
    utils::{Entry, HashMap},
};

pub use assets::Isosurface;
use compute::{CalculateIsosurfaceTasks, TaskInfo};

#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(Transform, Visibility)]
pub struct IsosurfaceHandle(pub Handle<Isosurface>);

#[derive(Default)]
pub struct IsosurfacePlugin;

#[derive(Resource, Default, DerefMut, Deref)]
struct MeshRegistry(HashMap<AssetId<Isosurface>, AssetId<Mesh>>);

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(assets::IsosurfaceAssetsPlugin)
            .add_plugins(compute::ComputeIsosurfacePlugin)
            .add_systems(
                PostUpdate,
                check_visibility::<With<IsosurfaceHandle>>
                    .in_set(VisibilitySystems::CheckVisibility),
            )
            .add_systems(PostUpdate, insert_phony_meshes)
            .init_resource::<MeshRegistry>();

        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, extract_isosurface);
    }
}

fn extract_isosurface(
    mut calculate_tasks: ResMut<CalculateIsosurfaceTasks>,
    isosurfaces: Extract<Query<(&IsosurfaceHandle, &Mesh3d)>>,
) {
    for (isosurface_handle, mesh_handle) in isosurfaces.iter() {
        if calculate_tasks.contains_key(&isosurface_handle.id()) {
            continue;
        }
        calculate_tasks.insert(
            isosurface_handle.id(),
            TaskInfo {
                mesh_id: mesh_handle.0.id(),
                done: false,
            },
        );
    }
}

fn insert_phony_meshes(
    mut commands: Commands,
    mut mesh_server: ResMut<Assets<Mesh>>,
    mut registry: ResMut<MeshRegistry>,
    isosurfaces: Query<(Entity, &IsosurfaceHandle), Without<Mesh3d>>,
) {
    for (entity, isosurface_handle) in isosurfaces.iter() {
        let handle = match registry.entry(isosurface_handle.id()) {
            Entry::Occupied(entry) => mesh_server.get_strong_handle(*entry.get()).unwrap(),
            Entry::Vacant(entry) => {
                let mut mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                );
                mesh.custom_allocation = true;
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
                let handle = mesh_server.add(mesh);
                entry.insert(handle.id());
                handle
            }
        };
        commands.entity(entity).insert(Mesh3d(handle));
    }
}
