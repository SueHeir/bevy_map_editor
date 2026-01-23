//! Stub system code generation
//!
//! Generates empty system function stubs for each placeable entity type,
//! providing a starting point for implementing entity behaviors.

use bevy_map_schema::Schema;
use codegen::Scope;

use crate::{format_code, to_snake_case, CodegenError};

/// Generate stub systems for all placeable entity types
pub fn generate_stubs(schema: &Schema) -> Result<String, CodegenError> {
    let placeable_types: Vec<_> = schema
        .data_types
        .iter()
        .filter(|(_, def)| def.placeable)
        .collect();

    if placeable_types.is_empty() {
        return Ok(generate_empty_stubs_module());
    }

    let mut scope = Scope::new();

    // Add imports (codegen puts these at the top automatically)
    scope.import("bevy::prelude", "*");
    scope.import("super::entities", "*");

    // Add comments (after imports in output)
    scope.raw("");
    scope.raw("// Auto-generated system stubs for entity types");
    scope.raw("// These stubs provide a starting point for implementing entity behaviors.");
    scope.raw("// You can move the implementations to your own files once you customize them.");
    scope.raw("//");
    scope.raw("// This file is regenerated when you save your map project.");
    scope.raw("// Move customized code to a separate file to preserve your changes.");
    scope.raw("");

    // Generate stubs for each placeable type
    let mut sorted_types: Vec<_> = placeable_types.iter().collect();
    sorted_types.sort_by_key(|(name, _)| *name);

    for (name, _type_def) in sorted_types {
        generate_entity_stubs(&mut scope, name);
    }

    // Generate plugin registration
    generate_stubs_plugin(&mut scope, &placeable_types);

    let code = scope.to_string();
    format_code(&code)
}

/// Generate stub systems for a single entity type
fn generate_entity_stubs(scope: &mut Scope, name: &str) {
    let snake_name = to_snake_case(name);

    // Update system - called every frame
    scope.raw(format!("/// Called every frame for {} entities", name));
    scope.raw("///");
    scope.raw("/// Use this to implement movement, AI, or other per-frame logic.");
    let update_fn = scope
        .new_fn(&format!("update_{}", snake_name))
        .vis("pub")
        .arg("_time", "Res<Time>")
        .arg(
            "_query",
            format!("Query<(Entity, &Transform, &{}), With<{}>>", name, name),
        );

    update_fn.line(format!("// TODO: Implement {} update logic", name));
    update_fn.line("// Example: Move entities, check conditions, update state");

    // Spawn callback - called when entity is spawned
    scope.raw("");
    scope.raw(format!("/// Called when a {} entity is spawned", name));
    scope.raw("///");
    scope.raw("/// Use this to set up additional components or initialize state.");
    let spawn_fn = scope
        .new_fn(&format!("on_{}_spawned", snake_name))
        .vis("pub")
        .arg("mut _commands", "Commands")
        .arg(
            "_query",
            format!("Query<(Entity, &Transform), Added<{}>>", name),
        );

    spawn_fn.line(format!("// TODO: Implement {} spawn setup", name));
    spawn_fn.line("// Example: Add physics components, start animations");

    // Despawn callback - called when entity is about to despawn
    scope.raw("");
    scope.raw(format!("/// Called when a {} entity is removed", name));
    scope.raw("///");
    scope.raw("/// Use this for cleanup or spawn effects on death/removal.");
    let despawn_fn = scope
        .new_fn(&format!("on_{}_removed", snake_name))
        .vis("pub")
        .arg("mut _commands", "Commands")
        .arg("_removed", format!("RemovedComponents<{}>", name));

    despawn_fn.line(format!("// TODO: Implement {} removal cleanup", name));
    despawn_fn.line("// Example: Spawn particles, drop items, play sounds");

    scope.raw("");
}

/// Generate the plugin that registers all stubs
fn generate_stubs_plugin(
    scope: &mut Scope,
    placeable_types: &[(&String, &bevy_map_schema::TypeDef)],
) {
    scope.raw("/// Plugin that registers all generated stub systems");
    scope.raw("///");
    scope.raw("/// Add this to your app to enable the stub systems:");
    scope.raw("/// ```ignore");
    scope.raw("/// app.add_plugins(StubsPlugin);");
    scope.raw("/// ```");

    let plugin = scope.new_struct("StubsPlugin").vis("pub");
    plugin.derive("Default");

    let impl_block = scope.new_impl("StubsPlugin");
    impl_block.impl_trait("Plugin");

    let mut build_body = String::from("app\n");

    for (name, _) in placeable_types {
        let snake = to_snake_case(name);
        build_body.push_str(&format!(
            "            .add_systems(Update, update_{})\n",
            snake
        ));
        build_body.push_str(&format!(
            "            .add_systems(Update, on_{}_spawned)\n",
            snake
        ));
        build_body.push_str(&format!(
            "            .add_systems(Update, on_{}_removed)\n",
            snake
        ));
    }
    build_body.push_str("            ;");

    let build_fn = impl_block
        .new_fn("build")
        .arg_ref_self()
        .arg("app", "&mut App");

    build_fn.line(build_body);
}

/// Generate an empty stubs module when there are no placeable types
fn generate_empty_stubs_module() -> String {
    r#"//! Auto-generated system stubs for entity types
//!
//! No placeable entity types are defined in this schema.

use bevy::prelude::*;

/// Empty plugin - no stubs to register
#[derive(Default)]
pub struct StubsPlugin;

impl Plugin for StubsPlugin {
    fn build(&self, _app: &mut App) {
        // No placeable entity types defined
    }
}
"#
    .to_string()
}

/// Generate individual stub file for a specific entity type
pub fn generate_entity_stub_file(name: &str) -> Result<String, CodegenError> {
    let mut scope = Scope::new();

    // Add imports (codegen puts these at the top automatically)
    scope.import("bevy::prelude", "*");
    scope.import("super::super::entities", name);

    // Add comments (after imports in output)
    scope.raw("");
    scope.raw(format!("// System stubs for {} entities", name));
    scope.raw("// Customize these functions to implement your entity's behavior.");
    scope.raw("");

    generate_entity_stubs(&mut scope, name);

    let code = scope.to_string();
    format_code(&code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_map_schema::TypeDef;

    fn make_test_schema() -> Schema {
        let mut schema = Schema::default();

        let mut player = TypeDef::default();
        player.placeable = true;
        schema.data_types.insert("Player".to_string(), player);

        let mut enemy = TypeDef::default();
        enemy.placeable = true;
        schema.data_types.insert("Enemy".to_string(), enemy);

        // Non-placeable type
        let item = TypeDef::default();
        schema.data_types.insert("Item".to_string(), item);

        schema
    }

    #[test]
    fn test_generate_stubs() {
        let schema = make_test_schema();
        let result = generate_stubs(&schema);
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.contains("update_player"));
        assert!(code.contains("update_enemy"));
        assert!(code.contains("on_player_spawned"));
        assert!(code.contains("on_enemy_spawned"));
        assert!(code.contains("StubsPlugin"));
        // Should NOT contain Item (not placeable)
        assert!(!code.contains("update_item"));
    }

    #[test]
    fn test_generate_empty_stubs() {
        let schema = Schema::default();
        let result = generate_stubs(&schema);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No placeable entity types"));
    }

    #[test]
    fn test_generate_entity_stub_file() {
        let result = generate_entity_stub_file("Player");
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.contains("update_player"));
        assert!(code.contains("on_player_spawned"));
    }
}
