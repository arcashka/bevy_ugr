mod assets;
mod compute;
mod draw;

use bevy::{
    pbr::{
        extract_meshes_for_cpu_building, ExtractMeshesSet, MeshFlags, MeshTransforms,
        NotShadowCaster, NotShadowReceiver, PreviousGlobalTransform, RenderMeshInstanceCpu,
        RenderMeshInstanceShared, RenderMeshInstances, TransmittedShadowReceiver,
    },
    prelude::*,
    render::{
        batching::NoAutomaticBatching,
        sync_world::MainEntity,
        view::{check_visibility, RenderVisibilityRanges, VisibilityRange, VisibilitySystems},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::{HashMap, HashSet, Parallel},
};

pub use assets::Isosurface;
use compute::CalculateIsosurfaceTasks;

#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(Transform, Visibility)]
pub struct IsosurfaceHandle(pub Handle<Isosurface>);

#[derive(Default, Resource, Deref, DerefMut)]
pub struct IsosurfaceInstances(HashMap<MainEntity, AssetId<Isosurface>>);

#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderIsosurfaceInstances(HashSet<MainEntity>);

#[derive(Default)]
pub struct IsosurfacePlugin;

impl Plugin for IsosurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(assets::IsosurfaceAssetsPlugin)
            .add_plugins(compute::ComputeIsosurfacePlugin)
            .add_plugins(draw::DrawIsosurfacePlugin)
            .add_systems(
                PostUpdate,
                check_visibility::<With<IsosurfaceHandle>>
                    .in_set(VisibilitySystems::CheckVisibility),
            );

        app.sub_app_mut(RenderApp)
            .add_systems(
                ExtractSchedule,
                (
                    extract_isosurface_instances,
                    extract_isosurfaces_for_render
                        .in_set(ExtractMeshesSet)
                        .after(extract_meshes_for_cpu_building),
                ),
            )
            .add_systems(
                Render,
                queue_isosurface_calculations.in_set(RenderSet::Queue),
            )
            .init_resource::<IsosurfaceInstances>()
            .init_resource::<RenderIsosurfaceInstances>();
    }
}

pub fn extract_isosurface_instances(
    mut isosurface_instances: ResMut<IsosurfaceInstances>,
    isosurfaces_query: Extract<Query<(Entity, &IsosurfaceHandle, &ViewVisibility)>>,
) {
    isosurface_instances.clear();
    for (entity, isosurface, view_visibility) in isosurfaces_query.iter() {
        if !view_visibility.get() {
            return;
        }
        isosurface_instances.insert(MainEntity::from(entity), isosurface.id());
    }
}

pub fn queue_isosurface_calculations(
    mut calculate_tasks: ResMut<CalculateIsosurfaceTasks>,
    isosurface_instances: Res<IsosurfaceInstances>,
) {
    for (_, asset_id) in isosurface_instances.iter() {
        if !calculate_tasks.contains_key(asset_id) {
            calculate_tasks.insert(*asset_id, false);
        }
    }
}

pub fn extract_isosurfaces_for_render(
    calculate_tasks: Res<CalculateIsosurfaceTasks>,
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    mut render_isosurface_instances: ResMut<RenderIsosurfaceInstances>,
    render_visibility_ranges: Res<RenderVisibilityRanges>,
    mut render_mesh_instance_queues: Local<Parallel<Vec<(Entity, RenderMeshInstanceCpu)>>>,
    query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            &IsosurfaceHandle,
            Has<NotShadowReceiver>,
            Has<TransmittedShadowReceiver>,
            Has<NotShadowCaster>,
            Has<NoAutomaticBatching>,
            Has<VisibilityRange>,
        )>,
    >,
) {
    query.par_iter().for_each_init(
        || render_mesh_instance_queues.borrow_local_mut(),
        |queue,
         (
            entity,
            view_visibility,
            transform,
            previous_transform,
            mesh,
            not_shadow_receiver,
            transmitted_receiver,
            not_shadow_caster,
            no_automatic_batching,
            visibility_range,
        )| {
            if !view_visibility.get() {
                return;
            }

            let Some(done) = calculate_tasks.get(&mesh.id()) else {
                return;
            };

            if !done {
                return;
            }

            let mut lod_index = None;
            if visibility_range {
                lod_index = render_visibility_ranges.lod_index_for_entity(entity.into());
            }

            let mesh_flags = MeshFlags::from_components(
                transform,
                lod_index,
                not_shadow_receiver,
                transmitted_receiver,
            );

            let shared = RenderMeshInstanceShared::from_components(
                previous_transform,
                mesh.id().untyped(),
                not_shadow_caster,
                no_automatic_batching,
            );

            let world_from_local = transform.affine();
            queue.push((
                entity,
                RenderMeshInstanceCpu {
                    transforms: MeshTransforms {
                        world_from_local: (&world_from_local).into(),
                        previous_world_from_local: (&previous_transform
                            .map(|t| t.0)
                            .unwrap_or(world_from_local))
                            .into(),
                        flags: mesh_flags.bits(),
                    },
                    shared,
                },
            ));
        },
    );

    let RenderMeshInstances::CpuBuilding(ref mut render_mesh_instances) = *render_mesh_instances
    else {
        panic!(
            "`extract_meshes_for_cpu_building` should only be called if we're using CPU \
            `MeshUniform` building"
        );
    };

    for queue in render_mesh_instance_queues.iter_mut() {
        for (entity, render_mesh_instance) in queue.drain(..) {
            let main_entity = MainEntity::from(entity);
            info!("add isosurface for render: {:?}", main_entity);
            render_mesh_instances.insert_unique_unchecked(main_entity, render_mesh_instance);
            render_isosurface_instances.insert(MainEntity::from(entity));
        }
    }
}
