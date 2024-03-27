use bevy::{prelude::*, render::render_resource::BindGroup};

#[derive(Default, Resource, Deref)]
pub struct DrawBindGroupLayout {
    pub model_only: Option<BindGroup>,
}

impl DrawBindGroupLayout {
    pub fn reset(&mut self) {
        self.model_only = None;
    }
}
