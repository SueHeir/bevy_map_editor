# Devlog Script: bevy_map_editor Quick Demo

**Target Length:** 6-7 minutes
**Goal:** Create a level, add collisions, add a player, run the game

---

## INTRO (20 seconds)

> "Today I'm showing bevy_map_editor - a 2D tilemap toolkit for Bevy.
>
> We'll set up a project from scratch, create a level, add collisions, place a player, and have a playable game in about 6 minutes."

---

## PART 0: PROJECT SETUP (45 seconds)

**[Screen: Terminal]**

> "Let's start from scratch. Create a new Rust project and add our dependencies:"

```bash
cargo new my_platformer
cd my_platformer
cargo add bevy
cargo add bevy_map --features physics
cargo add avian2d
```

> "That's it - Bevy, the map runtime with physics, and avian2d for collision. Your map files will go in `assets/maps/`. Now let's open the editor and create a level."

---

## PART 1: CREATE A LEVEL (1.5 minutes)

**[Screen: Editor running]**

> "Here's the editor. I'll import my tileset - just a PNG spritesheet with 16x16 tiles."

**[Action: Import tileset, paint a simple platformer level]**

> "I can paint tiles with the brush, and use the rectangle tool for filling areas.
>
> Let me quickly paint some ground, a few platforms, and some decoration."

---

## PART 2: ADD COLLISIONS (1 minute)

**[Screen: Tileset Editor → Collision tab]**

> "Now collisions. In the Tileset Editor, I select my ground tile and mark it as 'Full' collision.
>
> Same for the platforms. That's it - every instance of these tiles is now solid."

**[Action: Mark tiles as solid]**

> "I can also draw custom shapes for slopes or partial tiles, but full tile collision handles most cases."

---

## PART 3: ADD PLAYER ENTITY (45 seconds)

**[Screen: Entity layer]**

> "For the player, I switch to an Entity layer and place a 'Player' spawn point.
>
> I can set properties like speed right here in the editor."

**[Action: Place Player entity, set speed property]**

> "Save, and we're done with the editor side."

---

## PART 4: THE CODE (2 minutes)

**[Screen: Code editor]**

> "Now the game code. Here's everything we need:"

```rust
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::MapCollisionPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapCollisionPlugin)
        .register_map_entity::<Player>()
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_player_physics, player_movement))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(MapHandle(asset_server.load("maps/level1.map.json")));
}
```

> "MapHandle loads everything - tilemap, textures, collisions, entities.
>
> The Player component links to our editor entity:"

```rust
#[derive(Component, MapEntity)]
#[map_entity(type_name = "Player")]
pub struct Player {
    #[map_prop(default = 200.0)]
    pub speed: f32,
}
```

> "When a Player spawns, we give it physics:"

```rust
fn spawn_player_physics(
    mut commands: Commands,
    players: Query<Entity, Added<Player>>,
) {
    for entity in players.iter() {
        commands.entity(entity).insert((
            RigidBody::Dynamic,
            Collider::rectangle(14.0, 14.0),
            LockedAxes::ROTATION_LOCKED,
        ));
    }
}
```

> "And simple movement:"

```rust
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&Player, &mut LinearVelocity)>,
) {
    for (player, mut velocity) in players.iter_mut() {
        velocity.x = 0.0;
        if keyboard.pressed(KeyCode::KeyA) { velocity.x -= player.speed; }
        if keyboard.pressed(KeyCode::KeyD) { velocity.x += player.speed; }
        if keyboard.just_pressed(KeyCode::Space) { velocity.y = 300.0; }
    }
}
```

---

## PART 5: RUN IT (30 seconds)

**[Screen: Game running]**

> "And here it is - our level with working collisions and a controllable player.
>
> The collision shapes we defined in the editor are now real physics. A/D to move, Space to jump."

**[Action: Demonstrate player moving and colliding with platforms]**

---

## OUTRO (15 seconds)

> "That's bevy_map_editor. Visual editing, automatic collision generation, typed entities.
>
> Links in the description. Thanks for watching!"

---

## SHOTS NEEDED

1. Terminal: `cargo new` and `cargo add` commands
2. Editor: Import tileset
3. Editor: Paint level (can be time-lapse)
4. Editor: Mark tiles as solid in collision editor
5. Editor: Place Player entity
6. Code on screen (can use animated typing or highlights)
7. Game running with player moving

---

## FULL WORKING CODE

For reference, here's the complete `main.rs`:

```rust
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::MapCollisionPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapCollisionPlugin)
        .register_map_entity::<Player>()
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_player_physics, player_movement))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(MapHandle(asset_server.load("maps/level1.map.json")));
}

#[derive(Component, MapEntity)]
#[map_entity(type_name = "Player")]
pub struct Player {
    #[map_prop(default = 200.0)]
    pub speed: f32,
}

fn spawn_player_physics(mut commands: Commands, players: Query<Entity, Added<Player>>) {
    for entity in players.iter() {
        commands.entity(entity).insert((
            RigidBody::Dynamic,
            Collider::rectangle(14.0, 14.0),
            LockedAxes::ROTATION_LOCKED,
            GravityScale(1.0),
        ));
    }
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&Player, &mut LinearVelocity)>,
) {
    for (player, mut velocity) in players.iter_mut() {
        velocity.x = 0.0;
        if keyboard.pressed(KeyCode::KeyA) {
            velocity.x -= player.speed;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            velocity.x += player.speed;
        }
        if keyboard.just_pressed(KeyCode::Space) {
            velocity.y = 300.0;
        }
    }
}
```
