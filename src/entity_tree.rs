use bevy::prelude::*;
use bevy_egui::egui;
use egui::CollapsingHeader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default, Debug)]
pub struct EntityTree(HashMap<Entity, EntityTree>);

impl EntityTree {
  pub fn from_world(world: &mut World) -> Self {
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

  pub fn render(
    self,
    world_ref: Rc<RefCell<&mut World>>,
    ui: &mut egui::Ui,
    render_components: impl Fn(Entity, &mut egui::Ui) -> () + Copy,
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
          render_components(entity, ui);

          if subtree.0.len() > 0 {
            ui.label("Children");
            subtree.render(world_ref2, ui, render_components)
          }
        });
    }
  }
}
