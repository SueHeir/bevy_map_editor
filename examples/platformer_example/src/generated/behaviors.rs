use bevy::prelude::*;
use super::entities::*;
/// Platformer movement system for Player entities
///
/// Controls: A/D or Left/Right for horizontal movement, Space to jump
pub fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
) {
    for (mut _transform, mut velocity) in query.iter_mut() {
        let mut direction = 0.0;
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction += 1.0;
        }
        velocity.x = direction * 200.0;
        if keyboard.just_pressed(KeyCode::Space) {
            velocity.y = 400.0;
        }
    }
}
/// Plugin that registers all generated behavior systems
///
/// Add this to your app to enable the behavior systems:
/// ```ignore
/// app.add_plugins(BehaviorsPlugin);
/// ```
#[derive(Default)]
pub struct BehaviorsPlugin;
impl Plugin for BehaviorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, player_movement);
    }
}
