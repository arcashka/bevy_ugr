use bevy::{
    ecs::system::SystemState,
    prelude::*,
    render::{
        render_asset::{PrepareAssetError, PrepareNextFrameAssets, RenderAsset, RenderAssetUsages},
        MainWorld, Render, RenderApp, RenderSet,
    },
    utils::HashMap,
};

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

pub struct GpuIsosurface {
    pub grid_size: Vec3,
    pub grid_origin: Vec3,
    pub grid_density: UVec3,
}

impl RenderAsset for Isosurface {
    type PreparedAsset = GpuIsosurface;
    type Param = ();

    fn asset_usage(&self) -> RenderAssetUsages {
        self.asset_usage
    }

    fn prepare_asset(
        self,
        _: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, bevy::render::render_asset::PrepareAssetError<Self>> {
        Ok(GpuIsosurface {
            grid_size: self.grid_size,
            grid_origin: self.grid_origin,
            grid_density: self.grid_density,
        })
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IsosurfaceAssetsStorage(HashMap<AssetId<Isosurface>, GpuIsosurface>);

#[derive(Deref, DerefMut)]
pub struct AssetHandled(pub bool);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct NewIsosurfaceAssets(HashMap<AssetId<Isosurface>, AssetHandled>);

#[derive(Default)]
pub struct IsosurfaceAssetsPlugin;

#[derive(Resource, Default)]
struct ExtractedIsosurfaceAssets {
    changed_assets: Vec<(AssetId<Isosurface>, Isosurface)>,
}

impl Plugin for IsosurfaceAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CachedExtractIsosurfaceAssetSystemState>()
            .register_type::<Isosurface>()
            .init_asset::<Isosurface>()
            .register_asset_reflect::<Isosurface>();

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedIsosurfaceAssets>()
                .init_resource::<NewIsosurfaceAssets>()
                .init_resource::<PrepareNextFrameAssets<Isosurface>>()
                .init_resource::<IsosurfaceAssetsStorage>()
                .add_systems(ExtractSchedule, extract_isosurface_asset)
                .add_systems(
                    Render,
                    (
                        prepare_isosurface_assets.in_set(RenderSet::PrepareAssets),
                        cleanup_assets.in_set(RenderSet::Cleanup),
                    ),
                );
        }
    }
}

#[derive(Resource)]
struct CachedExtractIsosurfaceAssetSystemState {
    state: SystemState<(
        EventReader<'static, 'static, AssetEvent<Isosurface>>,
        Res<'static, Assets<Isosurface>>,
    )>,
}

impl FromWorld for CachedExtractIsosurfaceAssetSystemState {
    fn from_world(world: &mut World) -> Self {
        Self {
            state: SystemState::new(world),
        }
    }
}

fn extract_isosurface_asset(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    main_world.resource_scope(
        |world, mut cached_state: Mut<CachedExtractIsosurfaceAssetSystemState>| {
            let (mut events, assets) = cached_state.state.get_mut(world);

            let mut changed_assets = Vec::default();

            for event in events.read() {
                match event {
                    AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                        let Some(asset) = assets.get(*id) else {
                            return;
                        };
                        changed_assets.push((*id, asset.clone()));
                    }
                    _ => (),
                }
            }
            commands.insert_resource(ExtractedIsosurfaceAssets { changed_assets });
        },
    );
}

fn prepare_isosurface_assets(
    mut extracted_assets: ResMut<ExtractedIsosurfaceAssets>,
    mut new_assets: ResMut<NewIsosurfaceAssets>,
    mut storage: ResMut<IsosurfaceAssetsStorage>,
) {
    for (id, extracted_asset) in extracted_assets.changed_assets.drain(..) {
        match extracted_asset.prepare_asset(&mut ()) {
            Ok(prepared_asset) => {
                storage.insert(id, prepared_asset);
                new_assets.insert(id, AssetHandled(false));
            }
            Err(PrepareAssetError::RetryNextUpdate(_)) => {
                // not possible I think
                error!("Failed to extract asset: {:?}", id);
            }
        }
    }
}

fn cleanup_assets(mut new_assets: ResMut<NewIsosurfaceAssets>) {
    new_assets.retain(|_, handled| !handled.0);
}
