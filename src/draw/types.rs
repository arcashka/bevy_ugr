use bevy::{prelude::*, render::render_resource::BindGroup};

#[derive(Default, Resource, Deref)]
pub struct DrawBindGroups {
    pub model_only: Option<BindGroup>,
}

impl DrawBindGroups {
    pub fn reset(&mut self) {
        self.model_only = None;
    }
}

#[derive(Component)]
pub struct FakeMesh(pub Handle<Mesh>);
