use bevy::{
    ecs::system::{SystemParamItem, SystemState},
    prelude::*,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetUsages},
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

pub struct RenderIsosurface {
    pub grid_size: Vec3,
    pub grid_origin: Vec3,
    pub grid_density: UVec3,
}

impl RenderAsset for RenderIsosurface {
    type SourceAsset = Isosurface;
    type Param = ();

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(RenderIsosurface {
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

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IsosurfaceAssets(HashMap<AssetId<Isosurface>, RenderIsosurface>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct ReadyIsosurfaceAssets(HashMap<AssetId<Isosurface>, RenderIsosurface>);

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

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedIsosurfaceAssets>()
                .init_resource::<IsosurfaceAssets>()
                .add_systems(ExtractSchedule, extract_isosurface_asset)
                .add_systems(
                    Render,
                    (prepare_isosurface_assets.in_set(RenderSet::PrepareAssets),),
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

            let mut new_assets = Vec::default();

            for event in events.read() {
                match event {
                    AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                        let Some(asset) = assets.get(*id) else {
                            return;
                        };
                        new_assets.push((*id, asset.clone()));
                    }
                    _ => (),
                }
            }
            commands.insert_resource(ExtractedIsosurfaceAssets {
                changed_assets: new_assets,
            });
        },
    );
}

fn prepare_isosurface_assets(
    mut extracted_assets: ResMut<ExtractedIsosurfaceAssets>,
    mut storage: ResMut<IsosurfaceAssets>,
) {
    for (id, extracted_asset) in extracted_assets.changed_assets.drain(..) {
        match RenderIsosurface::prepare_asset(extracted_asset, &mut ()) {
            Ok(prepared_asset) => {
                storage.insert(id, prepared_asset);
            }
            Err(e) => {
                error!("error preparing asset: {}", e);
            }
        }
    }
}
