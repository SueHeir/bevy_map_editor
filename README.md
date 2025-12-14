# bevy_map_editor

A complete 2D tilemap editing ecosystem for Bevy 0.17. Create maps in the editor, load them at runtime with one line of code. For more 
complex asset loading, you can specify when to load those as well for more control.

<!-- TODO: Add editor screenshot here -->
![Editor Screenshot](docs/images/editor_screenshot.png)

## Features

- **Visual Map Editor** - egui-based editor with layer system, terrain painting, and entity placement
- **Autotiling** - Corner, Edge, and Mixed terrain modes using Wang tiles, currently WIP
- **Runtime Loading** - Efficient tilemap rendering via bevy_ecs_tilemap 0.17
- **Custom Entities** - Define game objects with `#[derive(MapEntity)]` proc macro
- **Sprite Animations** - Define sprite sheets with named animations, autoloaded at runtime
- **Dialogue Trees** - Visual node-based dialogue editor with branching conversations
- **Schema System** - Type-safe entity properties with validation

## Crates

| Crate                                           | Description                                         |
|-------------------------------------------------|-----------------------------------------------------|
| [bevy_map_core](crates/bevy_map_core)           | Core data types (Level, Layer, Tileset, MapProject) |
| [bevy_map_editor](crates/bevy_map_editor)       | Visual map editor with egui UI                      |
| [bevy_map_runtime](crates/bevy_map_runtime)     | Runtime rendering via bevy_ecs_tilemap              |
| [bevy_map_autotile](crates/bevy_map_autotile)   | Wang tile autotiling system - WIP                   |
| [bevy_map_animation](crates/bevy_map_animation) | Sprite sheet animations                             |
| [bevy_map_dialogue](crates/bevy_map_dialogue)   | Dialogue tree system                                |
| [bevy_map_derive](crates/bevy_map_derive)       | `#[derive(MapEntity)]` proc macro                   |
| [bevy_map_schema](crates/bevy_map_schema)       | Entity property validation                          |

## Quick Start

### Running the Editor

```rust
use bevy::prelude::*;
use bevy_map_editor::EditorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EditorPlugin)
        .run();
}
```

```bash
cargo run --example basic_editor -p bevy_map_editor_examples
```

### Loading Maps at Runtime

```rust
use bevy::prelude::*;
use bevy_map_runtime::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .add_systems(Startup, load_map)
        .run();
}

fn load_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // Load and spawn the map
    commands.spawn(MapHandle::new(
        asset_server.load("maps/level1.map.json"),
    ));
}
```

<!-- TODO: Add runtime screenshot here -->
![Runtime Screenshot](docs/images/runtime_screenshot.png)

### Defining Custom Entities

Define game entities in code, place them in the editor:

```rust
use bevy::prelude::*;
use bevy_map_derive::MapEntity;
use bevy_map_runtime::MapEntityRegistry;

#[derive(Component, MapEntity)]
#[map_entity(type_name = "NPC")]
pub struct Npc {
    #[map_prop]
    pub name: String,
    #[map_prop(default = 100)]
    pub health: i32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .register_map_entity::<Npc>()
        .run();
}
```

## Examples

| Example                 | Description                     |
|-------------------------|---------------------------------|
| `basic_editor`          | Full editor with all features   |
| `runtime_loader`        | Load and display a map          |
| `animation_auto_demo`   | Auto-loading animated sprites   |
| `animation_manual_demo` | Manual sprite animation control |
| `dialogue_auto_demo`    | Auto-loading dialogue trees     |
| `dialogue_manual_demo`  | Manual dialogue handling        |
| `custom_entities_demo`  | Custom entity types             |
| `tileset_demo`          | Tileset rendering               |

Run examples:
```bash
cargo run --example basic_editor -p bevy_map_editor_examples
cargo run --example runtime_loader -p bevy_map_editor_examples
```

## Map File Format

Maps are saved as `.map.json` files [see example full-project JSON](examples/assets/maps/example_project.map.json):

```json
{
  "version": 1,
  "schema": {
    "project": { "name": "My Game", "tile_size": 16 },
    "data_types": {
      "NPC": {
        "color": "#4CAF50",
        "placeable": true,
        "properties": [
          { "name": "name", "type": "string", "required": true },
          { "name": "health", "type": "int", "default": 100 }
        ]
      }
    }
  },
  "tilesets": [],
  "levels": [],
  "sprite_sheets": [],
  "dialogues": []
}
```

## Editor Features

<!-- TODO: Add feature screenshots here -->

### Terrain Painting
Autotile terrain transitions using Wang tiles (Corner, Edge, Mixed modes). This is heavily inspired by [](https://github.com/mapeditor/tiled)
and credit to @bjorn for helping me with the Tiled autotiling algorithm!

Autotiling is still a WIP while I get the algorithm right, but you can manually paint Tiles in the map currently.

![Terrain Painting](docs/images/terrain_painting.png)

### Entity Placement
Place custom entities with property editing in the inspector panel. As well as define custom Data Types for those entities.

![Entity Placement](docs/images/entity_placement.png)

### Dialogue Editor
Visual node-based dialogue tree editor with Text, Choice, Condition, and Action nodes. See [example](examples/dialogue/auto_demo.rs)

![Dialogue Editor](docs/images/dialogue_editor.png)

### Animation Editor
Define sprite sheets with multiple named animations per asset. See [example](examples/animation/auto_demo.rs).

![Animation Editor](docs/images/animation_editor.png)

## Keyboard Shortcuts

| Shortcut       | Action         |
|----------------|----------------|
| `Ctrl+N`       | New Project    |
| `Ctrl+O`       | Open Project   |
| `Ctrl+S`       | Save           |
| `Ctrl+Shift+S` | Save As        |
| `Ctrl+Z`       | Undo           |
| `Ctrl+Y`       | Redo           |
| `Ctrl+C/V/X`   | Copy/Paste/Cut |
| `G`            | Toggle Grid    |

## Compatibility

| Dependency       | Version |
|------------------|---------|
| Bevy             | 0.17    |
| bevy_ecs_tilemap | 0.17    |
| bevy_egui        | 0.38    |
| Rust             | 1.76+   |

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

## Contributing

Contributions welcome! Please open an issue or submit a pull request.
