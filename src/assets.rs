use std::marker::PhantomData;

use bevy::{
    ecs::system::{StaticSystemParam, SystemState},
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

#[derive(Resource, Deref, DerefMut)]
pub struct NewRenderAssets<A: RenderAsset>(HashMap<AssetId<A>, A::PreparedAsset>);

impl<A: RenderAsset> Default for NewRenderAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}

// similar to what standard RenderAssetPlugin does but NewRenderAssets<A>
// only includes RenderAssets which are newly added or modified
pub struct RenderAssetWatcherPlugin<A: RenderAsset> {
    phantom: PhantomData<A>,
}

impl Default for RenderAssetWatcherPlugin<Isosurface> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

#[derive(Resource)]
pub struct ExtractedAssets<A: RenderAsset> {
    changed_assets: Vec<(AssetId<A>, A)>,
}

impl<A: RenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            changed_assets: Default::default(),
        }
    }
}

impl<A: RenderAsset> Plugin for RenderAssetWatcherPlugin<A> {
    fn build(&self, app: &mut App) {
        app.init_resource::<CachedExtractRenderAssetSystemState<A>>()
            .register_type::<Isosurface>()
            .init_asset::<Isosurface>()
            .register_asset_reflect::<Isosurface>();

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedAssets<A>>()
                .init_resource::<NewRenderAssets<A>>()
                .init_resource::<PrepareNextFrameAssets<A>>()
                .add_systems(ExtractSchedule, extract_render_asset::<A>)
                .add_systems(Render, prepare_assets::<A>.in_set(RenderSet::PrepareAssets));
        }
    }
}

#[derive(Resource)]
struct CachedExtractRenderAssetSystemState<A: RenderAsset> {
    state: SystemState<(
        EventReader<'static, 'static, AssetEvent<A>>,
        Res<'static, Assets<A>>,
    )>,
}

impl<A: RenderAsset> FromWorld for CachedExtractRenderAssetSystemState<A> {
    fn from_world(world: &mut World) -> Self {
        Self {
            state: SystemState::new(world),
        }
    }
}

fn extract_render_asset<A: RenderAsset>(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    main_world.resource_scope(
        |world, mut cached_state: Mut<CachedExtractRenderAssetSystemState<A>>| {
            let (mut events, assets) = cached_state.state.get_mut(world);

            let mut changed_assets = Vec::default();

            for event in events.read() {
                #[allow(clippy::match_same_arms)]
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
            commands.insert_resource(ExtractedAssets { changed_assets });
        },
    );
}

pub fn prepare_assets<A: RenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<A>>,
    mut render_assets: ResMut<NewRenderAssets<A>>,
    param: StaticSystemParam<<A as RenderAsset>::Param>,
) {
    let mut param = param.into_inner();

    for (id, extracted_asset) in extracted_assets.changed_assets.drain(..) {
        match extracted_asset.prepare_asset(&mut param) {
            Ok(prepared_asset) => {
                render_assets.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(_)) => {
                // not possible I think
                error!("Failed to extract asset: {:?}", id);
            }
        }
    }
}
