# bevy_world_visualizer

A tool to visualize the state of the world. Shows all entities hierarchically based on parent-child relations, and shows all components of an entity. Uses [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui) to visualize entity components where applicable.

## Setup

Add the `bevy_world_visualizer::WorldVisualizerPlugin` to your app, then to show the the debugger do:

```rust
use bevy_world_visualizer::WorldVisualizerParams;
fn system(mut params: ResMut<WorldVisualizerParams>) {
  params.show = true;
}
```
