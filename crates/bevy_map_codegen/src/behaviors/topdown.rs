//! Top-down movement behavior generation
//!
//! Generates top-down style movement systems with 8-directional movement.

use bevy_map_core::InputConfig;
use codegen::Scope;

use crate::to_snake_case;

/// Generate a top-down movement system for an entity type
///
/// Returns the system function name for plugin registration.
pub fn generate_topdown_movement(
    scope: &mut Scope,
    entity_name: &str,
    config: &InputConfig,
) -> String {
    let snake_name = to_snake_case(entity_name);
    let fn_name = format!("{}_movement", snake_name);

    scope.raw("");
    scope.raw(format!(
        "/// Top-down movement system for {} entities",
        entity_name
    ));
    scope.raw("///");
    scope.raw("/// Controls: WASD or Arrow keys for 8-directional movement");

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

    let speed = config.speed;

    let mut body = String::new();

    body.push_str("for (mut _transform, mut velocity) in query.iter_mut() {\n");

    // Build movement direction
    body.push_str("        // Build movement direction from input\n");
    body.push_str("        let mut direction = Vec2::ZERO;\n\n");

    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {\n",
    );
    body.push_str("            direction.y += 1.0;\n");
    body.push_str("        }\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {\n",
    );
    body.push_str("            direction.y -= 1.0;\n");
    body.push_str("        }\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {\n",
    );
    body.push_str("            direction.x -= 1.0;\n");
    body.push_str("        }\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {\n",
    );
    body.push_str("            direction.x += 1.0;\n");
    body.push_str("        }\n\n");

    // Normalize and apply speed
    body.push_str("        // Normalize for consistent diagonal speed\n");
    body.push_str("        let direction = direction.normalize_or_zero();\n");
    body.push_str(&format!(
        "        let move_vec = direction * {:.1};\n",
        speed
    ));
    body.push_str("        velocity.x = move_vec.x;\n");
    body.push_str("        velocity.y = move_vec.y;\n");

    body.push_str("    }");

    f.line(body);

    fn_name
}

/// Generate a top-down movement system with smooth acceleration
pub fn generate_smooth_topdown_movement(
    scope: &mut Scope,
    entity_name: &str,
    config: &InputConfig,
) -> String {
    let snake_name = to_snake_case(entity_name);
    let fn_name = format!("{}_movement_smooth", snake_name);

    scope.raw("");
    scope.raw(format!(
        "/// Smooth top-down movement for {} with acceleration",
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

    let mut body = String::new();

    body.push_str("let dt = time.delta_secs();\n\n");
    body.push_str("    for (mut _transform, mut velocity) in query.iter_mut() {\n");

    // Build input direction
    body.push_str("        let mut input = Vec2::ZERO;\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {\n",
    );
    body.push_str("            input.y += 1.0;\n");
    body.push_str("        }\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {\n",
    );
    body.push_str("            input.y -= 1.0;\n");
    body.push_str("        }\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {\n",
    );
    body.push_str("            input.x -= 1.0;\n");
    body.push_str("        }\n");
    body.push_str(
        "        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {\n",
    );
    body.push_str("            input.x += 1.0;\n");
    body.push_str("        }\n");
    body.push_str("        let input = input.normalize_or_zero();\n\n");

    // Acceleration/deceleration
    body.push_str("        let current = Vec2::new(velocity.x, velocity.y);\n");
    body.push_str("        let new_vel = if input != Vec2::ZERO {\n");
    body.push_str(&format!("            let target = input * {:.1};\n", speed));
    body.push_str(&format!(
        "            let rate = {:.1} * dt;\n",
        speed / accel
    ));
    body.push_str("            current.move_towards(target, rate)\n");
    body.push_str("        } else {\n");
    body.push_str(&format!(
        "            let rate = {:.1} * dt;\n",
        speed / decel
    ));
    body.push_str("            current.move_towards(Vec2::ZERO, rate)\n");
    body.push_str("        };\n\n");

    body.push_str("        velocity.x = new_vel.x;\n");
    body.push_str("        velocity.y = new_vel.y;\n");

    body.push_str("    }");

    f.line(body);

    fn_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_map_core::InputProfile;

    #[test]
    fn test_generate_topdown_movement() {
        let mut scope = Scope::new();
        scope.import("bevy::prelude", "*");

        let config = InputConfig {
            profile: InputProfile::TopDown,
            speed: 150.0,
            jump_force: None,
            max_fall_speed: None,
            acceleration: 0.0,
            deceleration: 0.0,
        };

        let fn_name = generate_topdown_movement(&mut scope, "Enemy", &config);
        assert_eq!(fn_name, "enemy_movement");

        let code = scope.to_string();
        assert!(code.contains("fn enemy_movement"));
        assert!(code.contains("KeyCode::KeyW"));
        assert!(code.contains("150.0")); // speed
        assert!(code.contains("normalize_or_zero"));
    }
}
