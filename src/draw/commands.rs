use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    log::*,
    pbr::{SetMaterialBindGroup, SetMeshViewBindGroup},
    render::{
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::IndexFormat,
    },
};

use crate::{types::IsosurfaceBindGroups, IsosurfaceInstances};

pub type DrawIsosurfaceMaterial<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetIsosurfaceBindGroup<1>,
    SetMaterialBindGroup<M, 2>,
    DrawIsosurface,
);

pub struct DrawIsosurface;

impl<P: PhaseItem> RenderCommand<P> for DrawIsosurface {
    type Param = SRes<IsosurfaceInstances>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _: (),
        _: Option<()>,
        isosurface_instances: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let isosurface_instances = isosurface_instances.into_inner();
        let Some(isosurface) = isosurface_instances.get(&item.entity()) else {
            error!("isosurface instance not found");
            return RenderCommandResult::Failure;
        };
        let Some(vertex_buffer) = isosurface.vertex_buffer.as_ref() else {
            error!("vertex buffer does not exist");
            return RenderCommandResult::Failure;
        };
        let Some(index_buffer) = isosurface.index_buffer.as_ref() else {
            error!("index buffer does not exist");
            return RenderCommandResult::Failure;
        };
        let Some(indirect_buffer) = isosurface.indirect_buffer.as_ref() else {
            error!("indirect buffer does not exist");
            return RenderCommandResult::Failure;
        };
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), 0, IndexFormat::Uint32);
        pass.draw_indexed_indirect(indirect_buffer, 0);

        RenderCommandResult::Success
    }
}

pub struct SetIsosurfaceBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetIsosurfaceBindGroup<I> {
    type Param = SRes<IsosurfaceBindGroups>;
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
            error!("missing bind group. Should've been set in prepare_bind_group");
            return RenderCommandResult::Failure;
        };

        let dynamic_offsets: [u32; 3] = Default::default();
        let offset_count = 0;
        pass.set_bind_group(I, bind_group, &dynamic_offsets[0..offset_count]);

        RenderCommandResult::Success
    }
}
