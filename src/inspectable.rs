use crate::{entity_tree::EntityTree, WorldVisualizerParams};
use bevy::prelude::*;
use bevy::reflect::TypeRegistryArc;
use bevy_egui::egui;
use bevy_inspector_egui::{Context, Inspectable};
use egui::CollapsingHeader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// TODO: we should probably use serde for this, but that's a heavy dependency just to
// tokenize identifiers.
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

pub struct WorldWrapper<'a>(pub &'a mut World);

impl<'a> Inspectable for WorldWrapper<'a> {
  type Attributes = ();

  fn ui(&mut self, ui: &mut egui::Ui, _options: Self::Attributes, context: &Context) {
    let world = &mut *self.0;

    let resources = context.resources.as_ref().unwrap();

    let type_registry = resources.get::<TypeRegistryArc>().unwrap();
    let type_registry = type_registry.read();

    let params = resources.get::<WorldVisualizerParams>().unwrap();
    let inspect_generator = &params.inspect_generator;

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
      let mut ent_components = components[&entity]
        .iter()
        .map(|(entity_index, archetype_index, type_infos)| {
          type_infos.iter().map(move |type_info| {
            let type_name = type_info.type_name();
            let type_name_short = clean_path(type_name);
            (
              type_name_short,
              entity_index,
              archetype_index,
              type_info.clone(),
            )
          })
        })
        .flatten()
        .collect::<Vec<_>>();
      ent_components.sort_by_key(|(name, _, _, _)| name.clone());

      for (type_name_short, entity_index, archetype_index, type_info) in ent_components.into_iter()
      {
        let type_name = type_info.type_name();

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
                &type_info,
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
    });
  }
}
