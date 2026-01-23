# platformer_example

A game created with [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

## Running the Game

### Development (with hot-reload)

```bash
cargo run --features dev
```

With hot-reload enabled, save your map in the editor to see changes reflected in the running game!

### Release

```bash
cargo run --release
```

## Controls

- **WASD / Arrow Keys** - Pan camera
- **Q** - Zoom out
- **E** - Zoom in

## Project Structure

```
platformer_example/
├── Cargo.toml          # Project dependencies
├── src/
│   └── main.rs         # Game entry point
└── assets/
    └── maps/           # Map files (copied from editor)
        └── *.map.json
```

## Integration with bevy_map_editor

This project is designed to work with bevy_map_editor:

1. Open your map project in the editor
2. Go to **Project > Game Settings**
3. Set the game project path to this directory
4. Click **Run Game** to launch

The editor will:
- Copy your map file to `assets/maps/`
- Launch the game with hot-reload enabled
- Automatically update when you save in the editor

## Adding Game Logic

Modify `src/main.rs` to add your game logic:

- Add custom components and systems
- Handle player input
- Add physics (consider using `avian2d`)
- Implement game mechanics

## License

MIT OR Apache-2.0
