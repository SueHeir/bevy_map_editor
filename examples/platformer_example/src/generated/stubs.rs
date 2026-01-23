use bevy::prelude::*;
use super::entities::*;
/// Called every frame for Player entities
///
/// Use this to implement movement, AI, or other per-frame logic.
pub fn update_player(
    _time: Res<Time>,
    _query: Query<(Entity, &Transform, &Player), With<Player>>,
) {}
/// Called when a Player entity is spawned
///
/// Use this to set up additional components or initialize state.
pub fn on_player_spawned(
    mut _commands: Commands,
    _query: Query<(Entity, &Transform), Added<Player>>,
) {}
/// Called when a Player entity is removed
///
/// Use this for cleanup or spawn effects on death/removal.
pub fn on_player_removed(mut _commands: Commands, _removed: RemovedComponents<Player>) {}
/// Plugin that registers all generated stub systems
///
/// Add this to your app to enable the stub systems:
/// ```ignore
/// app.add_plugins(StubsPlugin);
/// ```
#[derive(Default)]
pub struct StubsPlugin;
impl Plugin for StubsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_player)
            .add_systems(Update, on_player_spawned)
            .add_systems(Update, on_player_removed);
    }
}
