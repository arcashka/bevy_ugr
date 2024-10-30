use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    log::*,
    pbr::{RenderMeshInstances, SetMaterialBindGroup, SetMeshViewBindGroup},
    render::{
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::IndexFormat,
    },
};

use crate::compute::{IndirectBuffersCollection, IsosurfaceBuffersCollection};

use super::types::DrawBindGroupLayout;

pub type DrawIsosurfaceMaterial<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetIsosurfaceBindGroup<1>,
    SetMaterialBindGroup<M, 2>,
    DrawIsosurface,
);

pub struct DrawIsosurface;

impl<P: PhaseItem> RenderCommand<P> for DrawIsosurface {
    type Param = (
        SRes<RenderMeshInstances>,
        SRes<IsosurfaceBuffersCollection>,
        SRes<IndirectBuffersCollection>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _: (),
        _: Option<()>,
        (mesh_instances, data_buffers_collection, indirect_buffers_collection): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data_buffers_collection = data_buffers_collection.into_inner();
        let indirect_buffers_collection = indirect_buffers_collection.into_inner();
        let mesh_instances = mesh_instances.into_inner();
        let Some(asset_id) = mesh_instances.mesh_asset_id(item.main_entity()) else {
            return RenderCommandResult::Failure("isosurface instance not found");
        };
        let (Some(data_buffers), Some(indirect_buffer)) = (
            data_buffers_collection.get(&isosurface.asset_id),
            indirect_buffers_collection.get(&isosurface.asset_id),
        ) else {
            error!(
                "isosurface buffers not found for asset {}, entity: {:?}",
                isosurface.asset_id,
                item.entity()
            );
            return RenderCommandResult::Success;
        };
        pass.set_vertex_buffer(0, data_buffers.vertex_buffer.slice(..));
        pass.set_index_buffer(data_buffers.index_buffer.slice(..), 0, IndexFormat::Uint32);
        pass.draw_indexed_indirect(&indirect_buffer.indirect_buffer, 0);
        info!("draw called");

        RenderCommandResult::Success
    }
}

pub struct SetIsosurfaceBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetIsosurfaceBindGroup<I> {
    type Param = SRes<DrawBindGroupLayout>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        _: Option<()>,
        bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = bind_groups.into_inner();

        let Some(bind_group) = bind_groups.model_only.as_ref() else {
            return RenderCommandResult::Failure(
                "missing bind group. Should've been set in prepare_bind_group",
            );
        };

        let dynamic_offsets: [u32; 3] = Default::default();
        let offset_count = 0;
        pass.set_bind_group(I, bind_group, &dynamic_offsets[0..offset_count]);

        RenderCommandResult::Success
    }
}
