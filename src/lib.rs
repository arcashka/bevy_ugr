mod assets;
mod compute;

use bevy::{
    asset::RenderAssetUsages,
    ecs::{schedule::And, system::SystemState},
    prelude::*,
    render::{
        mesh::{MeshVertexAttribute, PrimitiveTopology},
        view::{check_visibility, VisibilitySystems},
        Extract, MainWorld, RenderApp,
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

#[derive(Component, Deref, DerefMut)]
struct Mesh3dHolder(Mesh3d);

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(assets::IsosurfaceAssetsPlugin)
            .add_plugins(compute::ComputeIsosurfacePlugin)
            .add_systems(
                PostUpdate,
                check_visibility::<With<IsosurfaceHandle>>
                    .in_set(VisibilitySystems::CheckVisibility),
            )
            .add_systems(PostUpdate, create_temporary_meshes)
            .init_resource::<MeshRegistry>();

        app.sub_app_mut(RenderApp).add_systems(
            ExtractSchedule,
            (
                extract_isosurface,
                setup_real_meshes.after(extract_isosurface),
            ),
        );
    }
}

fn extract_isosurface(
    mut calculate_tasks: ResMut<CalculateIsosurfaceTasks>,
    isosurfaces: Extract<Query<(&IsosurfaceHandle, &Mesh3dHolder)>>,
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

fn setup_real_meshes(
    mut main_world: ResMut<MainWorld>,
    mut system_state: Local<
        Option<
            SystemState<(
                Query<(Entity, &IsosurfaceHandle, &Mesh3dHolder), Without<Mesh3d>>,
                ResMut<Assets<Mesh>>,
                Commands,
            )>,
        >,
    >,
    mut calculate_tasks: ResMut<CalculateIsosurfaceTasks>,
) {
    if system_state.is_none() {
        *system_state = Some(SystemState::new(&mut main_world));
    }
    let system_state = system_state.as_mut().unwrap();
    let (query, mut mesh_server, mut commands) = system_state.get_mut(&mut main_world);

    for (entity, isosurface_handle, mesh_handle) in query.iter() {
        match calculate_tasks.entry(isosurface_handle.id()) {
            Entry::Occupied(entry) => {
                if entry.get().done {
                    let mut mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::RENDER_WORLD,
                    );
                    mesh.custom_allocation = true;
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
                    mesh_server.insert(mesh_handle.id(), mesh);
                    // remove the temporary mesh and set the real one
                    commands
                        .entity(entity)
                        .insert(mesh_handle.0.clone())
                        .remove::<Mesh3dHolder>();

                    info!("Isosurface Mesh3d created");
                }
            }
            Entry::Vacant(_) => {
                unreachable!()
            }
        };
    }
    system_state.apply(&mut main_world);
}

fn create_temporary_meshes(
    mut commands: Commands,
    mut mesh_server: ResMut<Assets<Mesh>>,
    mut registry: ResMut<MeshRegistry>,
    isosurfaces: Query<(Entity, &IsosurfaceHandle), (Without<Mesh3d>, Without<Mesh3dHolder>)>,
) {
    for (entity, isosurface_handle) in isosurfaces.iter() {
        let handle = match registry.entry(isosurface_handle.id()) {
            Entry::Occupied(entry) => mesh_server.get_strong_handle(*entry.get()).unwrap(),
            Entry::Vacant(entry) => {
                let handle = mesh_server.reserve_handle();
                entry.insert(handle.id());
                handle
            }
        };
        commands.entity(entity).insert(Mesh3dHolder(Mesh3d(handle)));
    }
}
