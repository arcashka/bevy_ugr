use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{MaterialPipeline, MaterialPipelineKey, MeshUniform},
    prelude::*,
    render::{
        batching::GetBatchData,
        mesh::MeshVertexBufferLayout,
        render_resource::{
            RenderPipelineDescriptor, SpecializedMeshPipeline, SpecializedMeshPipelineError,
        },
    },
};

use std::hash::Hash;

use crate::IsosurfaceInstances;

#[derive(Resource)]
pub struct IsosurfaceMaterialPipeline<M: Material> {
    material_pipeline: MaterialPipeline<M>,
}

pub struct IsosurfaceMaterialPipelineKey<M: Material> {
    pub material_pipeline_key: MaterialPipelineKey<M>,
}

impl<M: Material> Eq for IsosurfaceMaterialPipelineKey<M> where M::Data: PartialEq {}

impl<M: Material> PartialEq for IsosurfaceMaterialPipelineKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.material_pipeline_key == other.material_pipeline_key
    }
}

impl<M: Material> Clone for IsosurfaceMaterialPipelineKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            material_pipeline_key: self.material_pipeline_key.clone(),
        }
    }
}

impl<M: Material> Hash for IsosurfaceMaterialPipelineKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.material_pipeline_key.hash(state);
    }
}

impl<M: Material> Clone for IsosurfaceMaterialPipeline<M> {
    fn clone(&self) -> Self {
        Self {
            material_pipeline: self.material_pipeline.clone(),
        }
    }
}

impl<M: Material> SpecializedMeshPipeline for IsosurfaceMaterialPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = IsosurfaceMaterialPipelineKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let descriptor = self
            .material_pipeline
            .specialize(key.material_pipeline_key, layout)?;
        Ok(descriptor)
    }
}

impl<M: Material> GetBatchData for IsosurfaceMaterialPipeline<M> {
    type Param = SRes<IsosurfaceInstances>;
    type CompareData = ();

    type BufferData = MeshUniform;

    fn get_batch_data(
        isosurface_instances: &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let isosurface = isosurface_instances.get(&entity)?;
        Some((MeshUniform::new(&isosurface.transforms, None), None))
    }
}

impl<M: Material> FromWorld for IsosurfaceMaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let material_pipeline = world.resource::<MaterialPipeline<M>>();
        Self {
            material_pipeline: material_pipeline.clone(),
        }
    }
}
