//! Behavior system code generation
//!
//! Generates pre-built systems for common 2D game patterns based on
//! entity type configurations.

pub mod health;
pub mod patrol;
pub mod platformer;
pub mod topdown;

use bevy_map_core::{EntityTypeConfig, InputProfile};
use bevy_map_schema::Schema;
use codegen::Scope;
use std::collections::HashMap;

use crate::{format_code, CodegenError};

/// Generate behavior systems based on entity type configurations
pub fn generate_behaviors(
    schema: &Schema,
    entity_configs: &HashMap<String, EntityTypeConfig>,
) -> Result<String, CodegenError> {
    let mut scope = Scope::new();

    // Add imports (codegen puts these at the top automatically)
    scope.import("bevy::prelude", "*");
    scope.import("super::entities", "*");

    // Add comments (after imports in output)
    scope.raw("");
    scope.raw("// Auto-generated behavior systems");
    scope.raw("// These systems are generated based on your entity type configurations.");
    scope.raw("// They provide pre-built movement, combat, and AI behaviors.");
    scope.raw("//");
    scope.raw("// This file is regenerated when you save your map project.");
    scope.raw("");

    let mut has_behaviors = false;
    let mut movement_systems: Vec<String> = Vec::new();
    let component_registrations: Vec<String> = Vec::new();

    // Generate behaviors for each entity type with input config
    for (type_name, type_def) in &schema.data_types {
        if !type_def.placeable {
            continue;
        }

        if let Some(config) = entity_configs.get(type_name) {
            // Generate movement behavior based on input profile
            if let Some(input_config) = &config.input {
                has_behaviors = true;

                match &input_config.profile {
                    InputProfile::Platformer => {
                        let system_name = platformer::generate_platformer_movement(
                            &mut scope,
                            type_name,
                            input_config,
                        );
                        movement_systems.push(system_name);
                    }
                    InputProfile::TopDown => {
                        let system_name =
                            topdown::generate_topdown_movement(&mut scope, type_name, input_config);
                        movement_systems.push(system_name);
                    }
                    InputProfile::TwinStick => {
                        // Twin-stick uses top-down movement + mouse aiming
                        let system_name =
                            topdown::generate_topdown_movement(&mut scope, type_name, input_config);
                        movement_systems.push(system_name);
                        // TODO: Add mouse aiming system
                    }
                    InputProfile::Custom { .. } | InputProfile::None => {
                        // No built-in behavior for custom or none
                    }
                }
            }
        }
    }

    // Generate health component and systems if any entity has it configured
    // (This would check for a "health" property or similar pattern)
    // For now, generate the health module separately

    if !has_behaviors {
        return Ok(generate_empty_behaviors_module());
    }

    // Generate the behaviors plugin
    generate_behaviors_plugin(&mut scope, &movement_systems, &component_registrations);

    let code = scope.to_string();
    format_code(&code)
}

/// Generate the plugin that registers all behavior systems
fn generate_behaviors_plugin(
    scope: &mut Scope,
    movement_systems: &[String],
    _component_registrations: &[String],
) {
    scope.raw("");
    scope.raw("/// Plugin that registers all generated behavior systems");
    scope.raw("///");
    scope.raw("/// Add this to your app to enable the behavior systems:");
    scope.raw("/// ```ignore");
    scope.raw("/// app.add_plugins(BehaviorsPlugin);");
    scope.raw("/// ```");

    let plugin = scope.new_struct("BehaviorsPlugin").vis("pub");
    plugin.derive("Default");

    let impl_block = scope.new_impl("BehaviorsPlugin");
    impl_block.impl_trait("Plugin");

    let mut build_body = String::from("app\n");

    for system in movement_systems {
        build_body.push_str(&format!("            .add_systems(Update, {})\n", system));
    }

    build_body.push_str("            ;");

    let build_fn = impl_block
        .new_fn("build")
        .arg_ref_self()
        .arg("app", "&mut App");

    build_fn.line(build_body);
}

/// Generate an empty behaviors module when there are no behaviors configured
fn generate_empty_behaviors_module() -> String {
    r#"//! Auto-generated behavior systems
//!
//! No behaviors are configured for any entity types.
//! Configure input profiles in the Schema Editor to generate movement behaviors.

use bevy::prelude::*;

/// Empty plugin - no behaviors to register
#[derive(Default)]
pub struct BehaviorsPlugin;

impl Plugin for BehaviorsPlugin {
    fn build(&self, _app: &mut App) {
        // No behaviors configured
    }
}
"#
    .to_string()
}

/// Generate health-related components and systems
pub fn generate_health_module() -> Result<String, CodegenError> {
    health::generate_health_systems()
}

/// Generate patrol AI components and systems
pub fn generate_patrol_module() -> Result<String, CodegenError> {
    patrol::generate_patrol_systems()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_map_core::{InputConfig, InputProfile};
    use bevy_map_schema::TypeDef;

    fn make_test_schema_and_configs() -> (Schema, HashMap<String, EntityTypeConfig>) {
        let mut schema = Schema::default();

        let mut player = TypeDef::default();
        player.placeable = true;
        schema.data_types.insert("Player".to_string(), player);

        let mut configs = HashMap::new();
        configs.insert(
            "Player".to_string(),
            EntityTypeConfig {
                input: Some(InputConfig {
                    profile: InputProfile::Platformer,
                    speed: 200.0,
                    jump_force: Some(400.0),
                    max_fall_speed: Some(600.0),
                    acceleration: 0.0,
                    deceleration: 0.0,
                }),
                ..Default::default()
            },
        );

        (schema, configs)
    }

    #[test]
    fn test_generate_behaviors() {
        let (schema, configs) = make_test_schema_and_configs();
        let result = generate_behaviors(&schema, &configs);
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.contains("player_movement"));
        assert!(code.contains("BehaviorsPlugin"));
    }

    #[test]
    fn test_generate_empty_behaviors() {
        let schema = Schema::default();
        let configs = HashMap::new();
        let result = generate_behaviors(&schema, &configs);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No behaviors are configured"));
    }
}
