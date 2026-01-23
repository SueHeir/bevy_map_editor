//! Patrol AI behavior generation
//!
//! Generates patrol components and systems for NPC movement patterns.

use crate::{format_code, CodegenError};
use codegen::Scope;

/// Generate patrol-related components and systems
pub fn generate_patrol_systems() -> Result<String, CodegenError> {
    let mut scope = Scope::new();

    // Add imports (codegen puts these at the top automatically)
    scope.import("bevy::prelude", "*");

    // Add comments (after imports in output)
    scope.raw("");
    scope.raw("// Patrol AI system components");
    scope.raw("// Provides waypoint-based patrol movement for NPCs and enemies.");
    scope.raw("");

    // Patrol component
    scope.raw("/// Patrol behavior component");
    scope.raw("///");
    scope.raw("/// Entities with this component will move between waypoints.");
    let patrol = scope
        .new_struct("Patrol")
        .vis("pub")
        .derive("Component")
        .derive("Debug")
        .derive("Clone");

    patrol.field("pub waypoints", "Vec<Vec2>");
    patrol.field("pub current_index", "usize");
    patrol.field("pub speed", "f32");
    patrol.field("pub wait_time", "f32");
    patrol.field("pub loop_mode", "PatrolLoopMode");

    // Patrol loop mode enum
    scope.raw("");
    scope.raw("/// How the patrol loops through waypoints");
    let loop_mode = scope
        .new_enum("PatrolLoopMode")
        .vis("pub")
        .derive("Debug")
        .derive("Clone")
        .derive("Copy")
        .derive("PartialEq")
        .derive("Eq")
        .derive("Default");

    loop_mode.new_variant("Loop").annotation("#[default]");
    loop_mode.new_variant("PingPong");
    loop_mode.new_variant("Once");

    // Patrol state component
    scope.raw("");
    scope.raw("/// Internal state for patrol movement");
    let state = scope
        .new_struct("PatrolState")
        .vis("pub")
        .derive("Component")
        .derive("Debug")
        .derive("Clone")
        .derive("Default");

    state.field("pub waiting", "bool");
    state.field("pub wait_timer", "f32");
    state.field("pub reverse", "bool");

    // Patrol impl
    scope.raw("");
    let patrol_impl = scope.new_impl("Patrol");

    patrol_impl
        .new_fn("new")
        .vis("pub")
        .arg("waypoints", "Vec<Vec2>")
        .arg("speed", "f32")
        .ret("Self")
        .line("Self {")
        .line("            waypoints,")
        .line("            current_index: 0,")
        .line("            speed,")
        .line("            wait_time: 0.0,")
        .line("            loop_mode: PatrolLoopMode::Loop,")
        .line("        }");

    patrol_impl
        .new_fn("with_wait_time")
        .vis("pub")
        .arg_self()
        .arg("seconds", "f32")
        .ret("Self")
        .line("Self { wait_time: seconds, ..self }");

    patrol_impl
        .new_fn("with_loop_mode")
        .vis("pub")
        .arg_self()
        .arg("mode", "PatrolLoopMode")
        .ret("Self")
        .line("Self { loop_mode: mode, ..self }");

    patrol_impl
        .new_fn("current_target")
        .vis("pub")
        .arg_ref_self()
        .ret("Option<Vec2>")
        .line("self.waypoints.get(self.current_index).copied()");

    // Patrol system
    scope.raw("");
    scope.raw("/// System that moves entities along their patrol paths");
    let patrol_fn = scope
        .new_fn("patrol_movement")
        .vis("pub")
        .arg("time", "Res<Time>")
        .arg(
            "mut query",
            "Query<(&mut Transform, &mut Patrol, &mut PatrolState)>",
        );

    let patrol_body = r#"let dt = time.delta_secs();

    for (mut transform, mut patrol, mut state) in query.iter_mut() {
        if patrol.waypoints.is_empty() {
            continue;
        }

        // Handle waiting at waypoint
        if state.waiting {
            state.wait_timer -= dt;
            if state.wait_timer <= 0.0 {
                state.waiting = false;
                advance_waypoint(&mut patrol, &mut state);
            }
            continue;
        }

        // Move towards current waypoint
        let Some(target) = patrol.current_target() else {
            continue;
        };

        let current = transform.translation.truncate();
        let direction = target - current;
        let distance = direction.length();

        if distance < 1.0 {
            // Reached waypoint
            transform.translation.x = target.x;
            transform.translation.y = target.y;

            if patrol.wait_time > 0.0 {
                state.waiting = true;
                state.wait_timer = patrol.wait_time;
            } else {
                advance_waypoint(&mut patrol, &mut state);
            }
        } else {
            // Move towards target
            let move_dist = (patrol.speed * dt).min(distance);
            let movement = direction.normalize() * move_dist;
            transform.translation.x += movement.x;
            transform.translation.y += movement.y;
        }
    }"#;

    patrol_fn.line(patrol_body);

    // Helper function for advancing waypoints
    scope.raw("");
    scope.raw("/// Advance to the next waypoint based on loop mode");

    let advance_fn = scope
        .new_fn("advance_waypoint")
        .arg("patrol", "&mut Patrol")
        .arg("state", "&mut PatrolState");

    let advance_body = r#"let len = patrol.waypoints.len();
    if len == 0 {
        return;
    }

    match patrol.loop_mode {
        PatrolLoopMode::Loop => {
            patrol.current_index = (patrol.current_index + 1) % len;
        }
        PatrolLoopMode::PingPong => {
            if state.reverse {
                if patrol.current_index == 0 {
                    state.reverse = false;
                    patrol.current_index = 1.min(len - 1);
                } else {
                    patrol.current_index -= 1;
                }
            } else {
                if patrol.current_index >= len - 1 {
                    state.reverse = true;
                    patrol.current_index = len.saturating_sub(2);
                } else {
                    patrol.current_index += 1;
                }
            }
        }
        PatrolLoopMode::Once => {
            if patrol.current_index < len - 1 {
                patrol.current_index += 1;
            }
        }
    }"#;

    advance_fn.line(advance_body);

    // Patrol plugin
    scope.raw("");
    scope.raw("/// Plugin that registers patrol systems");
    let _plugin = scope
        .new_struct("PatrolPlugin")
        .vis("pub")
        .derive("Default");

    let plugin_impl = scope.new_impl("PatrolPlugin");
    plugin_impl.impl_trait("Plugin");

    let build_fn = plugin_impl
        .new_fn("build")
        .arg_ref_self()
        .arg("app", "&mut App");

    build_fn.line("app.add_systems(Update, patrol_movement);");

    let code = scope.to_string();
    format_code(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_patrol_systems() {
        let result = generate_patrol_systems();
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.contains("struct Patrol"));
        assert!(code.contains("enum PatrolLoopMode"));
        assert!(code.contains("fn patrol_movement"));
        assert!(code.contains("fn advance_waypoint"));
        assert!(code.contains("PatrolPlugin"));
    }
}
