use bevy::reflect::{TypeRegistryArc, TypeRegistryInternal};
use bevy::{ecs::TypeInfo, prelude::*};
use bevy_egui::{egui, EguiContext};
use bevy_inspector_egui::{Inspectable};
use egui::CollapsingHeader;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default, Debug)]
struct EntityTree(HashMap<Entity, EntityTree>);

impl EntityTree {
  fn from_world(world: &mut World) -> Self {
    let mut tree = EntityTree::default();
    let mut paths = HashMap::new();

    for entity in world.query_filtered::<Entity, Without<Parent>>() {
      tree.0.insert(entity, EntityTree::default());
      paths.insert(entity, vec![entity]);
    }

    loop {
      let mut changed = false;
      for (child, parent) in world.query::<(Entity, &Parent)>() {
        // If we've already processed the child, ignore
        if paths.contains_key(&child) {
          continue;
        }

        // If the child's parent has been processed, then process teh child
        if let Some(path) = paths.get(parent) {
          // Follow path to point in tree
          let level = path.iter().fold(&mut tree, |tree, path_ent| {
            tree.0.get_mut(path_ent).unwrap()
          });

          // Add child at that level of the tree
          level.0.insert(child, EntityTree::default());

          // Create a path for the inserted child
          let mut child_path = path.clone();
          child_path.push(child);
          paths.insert(child, child_path);

          // Register the change
          changed = true;
        }
      }
      if !changed {
        break;
      }
    }

    tree
  }

  fn render(
    self,
    world_ref: Rc<RefCell<&mut World>>,
    ui: &mut egui::Ui,
    render: impl Fn(Entity, &mut egui::Ui) -> () + Copy,
  ) {
    let mut nodes = self.0.into_iter().collect::<Vec<_>>();
    nodes.sort_by_key(|(k, _)| k.id());

    for (entity, subtree) in nodes.into_iter() {
      let label = {
        let world = world_ref.borrow();
        if let Ok(name) = world.get::<Name>(entity) {
          format!("Entity {} - {}", entity.id(), name.as_str())
        } else {
          format!("Entity {}", entity.id())
        }
      };

      let world_ref2 = world_ref.clone();
      CollapsingHeader::new(label)
        .default_open(false)
        .show(ui, |ui| {
          ui.label("Components");
          render(entity, ui);

          if subtree.0.len() > 0 {
            ui.label("Children");
            subtree.render(world_ref2, ui, render)
          }
        });
    }
  }
}

type InspectCallback = Box<dyn Fn(*mut u8, &mut egui::Ui, &Resources) -> ()>;

use std::mem;

macro_rules! ui_for_type {
  ($t:ty) => {
    (
      TypeId::of::<$t>(),
      Box::new(|ptr: *mut u8, ui: &mut egui::Ui, resources: &Resources| {
        let value: &mut $t = unsafe { mem::transmute(ptr) };
        value.ui(
          ui,
          <$t as Inspectable>::Attributes::from_resources(resources),
        )
      }) as InspectCallback,
    )
  };
}

macro_rules! ui_for_types {
  ($($t:ty),*) => {
    vec![$(ui_for_type!($t)),*]
  }
}

fn clean_path(path: &str) -> String {
  // Remove leading paths to in component names
  let re = regex::Regex::new(r"([\w\d]+)(<([^>]+)>)?$").unwrap();
  match re.captures(path) {
    Some(captures) => {
      let base = captures.get(1).unwrap().as_str().to_string();
      match captures.get(3) {
        Some(param) => {
          format!("{}<{}>", base, param.as_str().split("::").last().unwrap())
        }
        None => base,
      }
    }
    None => path.to_string(),
  }
}

struct InspectGenerator {
  impls: HashMap<TypeId, InspectCallback>,
}

impl InspectGenerator {
  fn new() -> Self {
    let mut impls = ui_for_types!(Transform, GlobalTransform);

    #[cfg(feature = "rapier")]
    {
      use bevy_rapier3d::physics::RigidBodyHandleComponent;
      impls.extend(
        ui_for_types!(RigidBodyHandleComponent)
      );
    }
        
    InspectGenerator {
      impls: impls.into_iter().collect::<HashMap<_, _>>()
    }
  }

  fn generate(
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
    let ptr = unsafe { archetype.get_dynamic(type_info.id(), type_info.layout().size(), entity_index).unwrap().as_ptr() };  
    if let Some(f) = self.impls.get(&type_info.id()) {
      f(ptr, ui, resources);
    } else {
      let registration = type_registry.get(type_info.id())?;
      let reflect_component = registration.data::<ReflectComponent>()?;
      let reflected = unsafe {
        reflect_component.reflect_component_mut(&world.archetypes[archetype_index], entity_index)
      };
      bevy_inspector_egui::reflect::ui_for_reflect(reflected, ui);
    };
   
    Some(())
  }
}

#[derive(Default)]
pub struct WorldVisualizerParams {
  pub show: bool,
}

fn world_visualizer_system(world: &mut World, resources: &mut Resources) {
  let params = resources.get::<WorldVisualizerParams>().unwrap();
  if !params.show {
    return;
  }

  let mut egui_context = resources.get_mut::<EguiContext>().unwrap();
  let ctx = &mut egui_context.ctx;

  let type_registry = resources.get::<TypeRegistryArc>().unwrap();
  let type_registry = type_registry.read();

  let inspect_generator = InspectGenerator::new();

  egui::Window::new("World Visualizer")
    .scroll(true)
    .show(ctx, |ui| {
      let mut components = HashMap::new();
      for (archetype_index, archetype) in world.archetypes().enumerate() {
        for (entity_index, entity) in archetype.iter_entities().enumerate() {
          let cs = components.entry(*entity).or_insert_with(Vec::new);
          cs.push((
            entity_index,
            archetype_index,
            archetype.types().iter().cloned().collect::<Vec<_>>(),
          ));
        }
      }

      let tree = EntityTree::from_world(world);
      let world_ref = Rc::new(RefCell::new(world));

      tree.render(world_ref.clone(), ui, |entity, ui| {
        for (entity_index, archetype_index, type_infos) in &components[&entity] {
          for type_info in type_infos.iter() {
            let type_name = type_info.type_name();
            let type_name_short = clean_path(type_name);

            // Skip over components that are implicitly visualized in the UI already: names and hierarchy
            let ignore = vec![
              "bevy_core::name::Name",
              "bevy_transform::components::children::Children",
              "bevy_transform::components::parent::Parent",
              "bevy_transform::components::parent::PreviousParent",
            ];
            if ignore.iter().find(|name| *name == &type_name).is_some() {
              continue;
            }

            CollapsingHeader::new(type_name_short)
              .default_open(false)
              .show(ui, |ui| {
                let world = world_ref.borrow();
                if inspect_generator
                  .generate(
                    &world,
                    &resources,
                    *archetype_index,
                    *entity_index,
                    type_info,
                    &*type_registry,
                    ui,
                  )
                  .is_none()
                {
                  ui.label("Inspectable has not been defined for this component");
                }
              })
              .header_response
              .on_hover_text(type_name);
          }
        }
      });
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
