mod compute;

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParamItem,
    prelude::*,
    render::{
        mesh::PrimitiveTopology,
        render_asset::{ExtractedAssets, PrepareAssetError, RenderAsset, RenderAssetPlugin},
        view::{check_visibility, VisibilitySystems},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::{Entry, HashMap},
};

use compute::CalculateIsosurfaceTasks;

#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(Transform, Visibility)]
pub struct IsosurfaceHandle(pub Handle<Isosurface>);

#[derive(Default)]
pub struct IsosurfacePlugin;

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderAssetPlugin::<ComputeIsosurface>::default())
            .add_plugins(compute::ComputeIsosurfacePlugin)
            .init_asset::<Isosurface>()
            .add_systems(
                PostUpdate,
                check_visibility::<With<IsosurfaceHandle>>
                    .in_set(VisibilitySystems::CheckVisibility),
            )
            .add_systems(PostUpdate, insert_phony_meshes)
            .init_resource::<MeshRegistry>();

        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, fill_render_mesh_registry)
            .add_systems(Render, schedule_isosurface_tasks.in_set(RenderSet::Queue))
            .init_resource::<MeshRegistry>();
    }
}

#[derive(Resource, Default, DerefMut, Deref)]
struct MeshRegistry(HashMap<AssetId<Isosurface>, AssetId<Mesh>>);

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

pub struct ComputeIsosurface {
    pub grid_size: Vec3,
    pub grid_origin: Vec3,
    pub grid_density: UVec3,
}

impl RenderAsset for ComputeIsosurface {
    type SourceAsset = Isosurface;
    type Param = ();

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        info!("preparing isosurface asset");
        Ok(ComputeIsosurface {
            grid_size: source_asset.grid_size,
            grid_origin: source_asset.grid_origin,
            grid_density: source_asset.grid_density,
        })
    }

    #[inline]
    fn asset_usage(mesh: &Self::SourceAsset) -> RenderAssetUsages {
        mesh.asset_usage
    }
}

fn fill_render_mesh_registry(
    mut mesh_registry: ResMut<MeshRegistry>,
    isosurfaces: Extract<Query<(&IsosurfaceHandle, &Mesh3d)>>,
) {
    for (isosurface_handle, mesh_handle) in isosurfaces.iter() {
        if mesh_registry.contains_key(&isosurface_handle.id()) {
            continue;
        }
        mesh_registry.insert(isosurface_handle.id(), mesh_handle.id());
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

fn schedule_isosurface_tasks(
    extracted_meshes: Res<ExtractedAssets<ComputeIsosurface>>,
    mesh_registry: Res<MeshRegistry>,
    mut tasks: ResMut<CalculateIsosurfaceTasks>,
) {
    for id in extracted_meshes.added.iter() {
        let mesh_id = mesh_registry.get(id).unwrap();
        info!("scheduling task for isosurface {}", id);
        tasks.insert(*id, *mesh_id);
    }
}
