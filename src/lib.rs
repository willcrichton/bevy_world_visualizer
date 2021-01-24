use crate::{inspect_generator::InspectGenerator, inspectable::WorldWrapper};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};
use bevy_inspector_egui::{Context, Inspectable};

mod entity_tree;
mod inspect_generator;
mod inspectable;

#[derive(Default)]
pub struct WorldVisualizerParams {
  pub show: bool,
  pub inspect_generator: InspectGenerator
}

fn world_visualizer_system(world: &mut World, resources: &mut Resources) {
  let params = resources.get::<WorldVisualizerParams>().unwrap();
  if !params.show {
    return;
  }

  let mut egui_context = resources.get_mut::<EguiContext>().unwrap();
  let ctx = &mut egui_context.ctx;

  egui::Window::new("World Visualizer")
    .scroll(true)
    .show(ctx, |ui| {
      WorldWrapper(world).ui(ui, Default::default(), &Context::new(resources))
    });
}

pub struct WorldVisualizerPlugin;
impl Plugin for WorldVisualizerPlugin {
  fn build(&self, app: &mut AppBuilder) {
    app
      .init_resource::<WorldVisualizerParams>()
      .add_system(world_visualizer_system.system());
  }
}
