//! Platformer movement behavior generation
//!
//! Generates platformer-style movement systems with horizontal movement,
//! jumping, and gravity handling.

use bevy_map_core::InputConfig;
use codegen::Scope;

use crate::to_snake_case;

/// Generate a platformer movement system for an entity type
///
/// Returns the system function name for plugin registration.
pub fn generate_platformer_movement(
    scope: &mut Scope,
    entity_name: &str,
    config: &InputConfig,
) -> String {
    let snake_name = to_snake_case(entity_name);
    let fn_name = format!("{}_movement", snake_name);

    scope.raw("");
    scope.raw(format!(
        "/// Platformer movement system for {} entities",
        entity_name
    ));
    scope.raw("///");
    scope.raw("/// Controls: A/D or Left/Right for horizontal movement, Space to jump");

    let f = scope
        .new_fn(&fn_name)
        .vis("pub")
        .arg("keyboard", "Res<ButtonInput<KeyCode>>")
        .arg(
            "mut query",
            format!(
                "Query<(&mut Transform, &mut LinearVelocity), With<{}>>",
                entity_name
            ),
        );

    // Extract config values
    let speed = config.speed;
    let jump_force = config.jump_force.unwrap_or(400.0);
    let _max_fall = config.max_fall_speed.unwrap_or(600.0);

    // Build the system body
    let mut body = String::new();

    body.push_str("for (mut _transform, mut velocity) in query.iter_mut() {\n");

    // Horizontal movement
    body.push_str("        // Horizontal movement\n");
    body.push_str("        let mut direction = 0.0;\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {\n",
    );
    body.push_str("            direction -= 1.0;\n");
    body.push_str("        }\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {\n",
    );
    body.push_str("            direction += 1.0;\n");
    body.push_str("        }\n");
    body.push_str(&format!("        velocity.x = direction * {:.1};\n", speed));

    // Jump
    body.push_str("\n        // Jumping\n");
    body.push_str("        // Note: You may want to add ground detection logic here\n");
    body.push_str("        if keyboard.just_pressed(KeyCode::Space) {\n");
    body.push_str(&format!("            velocity.y = {:.1};\n", jump_force));
    body.push_str("        }\n");

    body.push_str("    }");

    f.line(body);

    fn_name
}

/// Generate a more advanced platformer controller with acceleration
pub fn generate_advanced_platformer_movement(
    scope: &mut Scope,
    entity_name: &str,
    config: &InputConfig,
) -> String {
    let snake_name = to_snake_case(entity_name);
    let fn_name = format!("{}_movement_advanced", snake_name);

    scope.raw("");
    scope.raw(format!(
        "/// Advanced platformer movement for {} with acceleration",
        entity_name
    ));

    let f = scope
        .new_fn(&fn_name)
        .vis("pub")
        .arg("time", "Res<Time>")
        .arg("keyboard", "Res<ButtonInput<KeyCode>>")
        .arg(
            "mut query",
            format!(
                "Query<(&mut Transform, &mut LinearVelocity), With<{}>>",
                entity_name
            ),
        );

    let speed = config.speed;
    let accel = config.acceleration.max(0.1);
    let decel = config.deceleration.max(0.1);
    let jump_force = config.jump_force.unwrap_or(400.0);

    let mut body = String::new();

    body.push_str("let dt = time.delta_secs();\n");
    body.push('\n');
    body.push_str("    for (mut _transform, mut velocity) in query.iter_mut() {\n");

    // Input
    body.push_str("        // Get input direction\n");
    body.push_str("        let mut input_dir = 0.0;\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {\n",
    );
    body.push_str("            input_dir -= 1.0;\n");
    body.push_str("        }\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {\n",
    );
    body.push_str("            input_dir += 1.0;\n");
    body.push_str("        }\n\n");

    // Acceleration/deceleration
    body.push_str("        // Apply acceleration or deceleration\n");
    body.push_str("        if input_dir != 0.0 {\n");
    body.push_str(&format!(
        "            let target = input_dir * {:.1};\n",
        speed
    ));
    body.push_str(&format!(
        "            let accel_rate = {:.1} * dt;\n",
        speed / accel
    ));
    body.push_str("            velocity.x = velocity.x + (target - velocity.x).clamp(-accel_rate, accel_rate);\n");
    body.push_str("        } else {\n");
    body.push_str(&format!(
        "            let decel_rate = {:.1} * dt;\n",
        speed / decel
    ));
    body.push_str("            if velocity.x.abs() < decel_rate {\n");
    body.push_str("                velocity.x = 0.0;\n");
    body.push_str("            } else {\n");
    body.push_str("                velocity.x -= velocity.x.signum() * decel_rate;\n");
    body.push_str("            }\n");
    body.push_str("        }\n\n");

    // Jump
    body.push_str("        // Jump\n");
    body.push_str("        if keyboard.just_pressed(KeyCode::Space) {\n");
    body.push_str(&format!("            velocity.y = {:.1};\n", jump_force));
    body.push_str("        }\n");

    body.push_str("    }");

    f.line(body);

    fn_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_map_core::InputProfile;

    #[test]
    fn test_generate_platformer_movement() {
        let mut scope = Scope::new();
        scope.import("bevy::prelude", "*");

        let config = InputConfig {
            profile: InputProfile::Platformer,
            speed: 200.0,
            jump_force: Some(400.0),
            max_fall_speed: Some(600.0),
            acceleration: 0.0,
            deceleration: 0.0,
        };

        let fn_name = generate_platformer_movement(&mut scope, "Player", &config);
        assert_eq!(fn_name, "player_movement");

        let code = scope.to_string();
        assert!(code.contains("fn player_movement"));
        assert!(code.contains("KeyCode::Space"));
        assert!(code.contains("200.0")); // speed
        assert!(code.contains("400.0")); // jump force
    }
}
