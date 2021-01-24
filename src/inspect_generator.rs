use bevy::reflect::TypeRegistryInternal;
use bevy::{ecs::TypeInfo, prelude::*};
use bevy_egui::egui;
use bevy_inspector_egui::{Context, Inspectable};
use std::any::TypeId;
use std::collections::HashMap;
use std::mem;

pub type InspectCallback = Box<dyn Fn(*mut u8, &mut egui::Ui, &Resources) -> () + Send + Sync>;

pub struct InspectGenerator {
  impls: HashMap<TypeId, InspectCallback>,
}

impl Default for InspectGenerator {
  fn default() -> Self {
    let mut this = InspectGenerator {
      impls: HashMap::new(),
    };

    this.register::<Transform>();
    //this.register::<GlobalTransform>();

    // #[cfg(feature = "rapier")]
    // {
    //   use bevy_rapier3d::physics::RigidBodyHandleComponent;
    //   this.register::<RigidBodyHandleComponent>();
    // }

    this
  }
}

impl InspectGenerator {
  pub fn register<T: Inspectable + 'static>(&mut self) {
    let type_id = TypeId::of::<T>();
    let generator = Box::new(|ptr: *mut u8, ui: &mut egui::Ui, resources: &Resources| {
      let value: &mut T = unsafe { mem::transmute(ptr) };
      value.ui(
        ui,
        <T as Inspectable>::Attributes::default(),
        &Context::new(resources),
      )
    }) as InspectCallback;
    self.impls.insert(type_id, generator);
  }

  pub fn generate(
    &self,
    world: &World,
    resources: &Resources,
    archetype_index: usize,
    entity_index: usize,
    type_info: &TypeInfo,
    type_registry: &TypeRegistryInternal,
    ui: &mut egui::Ui,
  ) -> Option<()> {
    let archetype = &world.archetypes[archetype_index];

    let ptr = unsafe {
      archetype
        .get_dynamic(type_info.id(), type_info.layout().size(), entity_index)
        .unwrap()
        .as_ptr()
    };
    if let Some(f) = self.impls.get(&type_info.id()) {
      f(ptr, ui, resources);
    } else {
      let registration = type_registry.get(type_info.id())?;
      let reflect_component = registration.data::<ReflectComponent>()?;
      let reflected = unsafe { reflect_component.reflect_component_mut(archetype, entity_index) };
      bevy_inspector_egui::reflect::ui_for_reflect(reflected, ui, &Context::new(resources));
    };

    Some(())
  }
}
