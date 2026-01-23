//! Health system behavior generation
//!
//! Generates health components and damage/death systems for entities.

use crate::{format_code, CodegenError};
use codegen::Scope;

/// Generate health-related components and systems
pub fn generate_health_systems() -> Result<String, CodegenError> {
    let mut scope = Scope::new();

    // Add imports (codegen puts these at the top automatically)
    scope.import("bevy::prelude", "*");

    // Add comments (after imports in output)
    scope.raw("");
    scope.raw("// Health system components and events");
    scope.raw("// Provides health management with damage and death handling.");
    scope.raw("");

    // Health component
    scope.raw("/// Health component for entities that can take damage");
    let health = scope
        .new_struct("Health")
        .vis("pub")
        .derive("Component")
        .derive("Debug")
        .derive("Clone");

    health.field("pub current", "f32");
    health.field("pub max", "f32");

    // Health impl
    let health_impl = scope.new_impl("Health");

    health_impl
        .new_fn("new")
        .vis("pub")
        .arg("max", "f32")
        .ret("Self")
        .line("Self { current: max, max }");

    health_impl
        .new_fn("take_damage")
        .vis("pub")
        .arg_mut_self()
        .arg("amount", "f32")
        .line("self.current = (self.current - amount).max(0.0);");

    health_impl
        .new_fn("heal")
        .vis("pub")
        .arg_mut_self()
        .arg("amount", "f32")
        .line("self.current = (self.current + amount).min(self.max);");

    health_impl
        .new_fn("is_dead")
        .vis("pub")
        .arg_ref_self()
        .ret("bool")
        .line("self.current <= 0.0");

    health_impl
        .new_fn("percentage")
        .vis("pub")
        .arg_ref_self()
        .ret("f32")
        .line("self.current / self.max");

    // Damage event
    scope.raw("");
    scope.raw("/// Event fired when an entity takes damage");
    let damage_event = scope
        .new_struct("DamageEvent")
        .vis("pub")
        .derive("Event")
        .derive("Debug")
        .derive("Clone");

    damage_event.field("pub target", "Entity");
    damage_event.field("pub amount", "f32");
    damage_event.field("pub source", "Option<Entity>");

    // Death event
    scope.raw("");
    scope.raw("/// Event fired when an entity dies (health reaches zero)");
    let death_event = scope
        .new_struct("DeathEvent")
        .vis("pub")
        .derive("Event")
        .derive("Debug")
        .derive("Clone");

    death_event.field("pub entity", "Entity");
    death_event.field("pub killed_by", "Option<Entity>");

    // Damage processing system
    scope.raw("");
    scope.raw("/// System that processes damage events and updates health");
    let process_damage = scope
        .new_fn("process_damage")
        .vis("pub")
        .arg("mut damage_events", "EventReader<DamageEvent>")
        .arg("mut death_events", "EventWriter<DeathEvent>")
        .arg("mut health_query", "Query<&mut Health>");

    let damage_body = r#"for event in damage_events.read() {
        if let Ok(mut health) = health_query.get_mut(event.target) {
            health.take_damage(event.amount);

            if health.is_dead() {
                death_events.send(DeathEvent {
                    entity: event.target,
                    killed_by: event.source,
                });
            }
        }
    }"#;

    process_damage.line(damage_body);

    // Death processing system (stub)
    scope.raw("");
    scope.raw("/// System that handles entity death");
    scope.raw("///");
    scope.raw("/// Override this to customize death behavior (despawn, respawn, etc.)");
    let handle_death = scope
        .new_fn("handle_death")
        .vis("pub")
        .arg("mut commands", "Commands")
        .arg("mut death_events", "EventReader<DeathEvent>");

    handle_death.line("for event in death_events.read() {");
    handle_death.line("        // Default behavior: despawn the entity");
    handle_death.line("        commands.entity(event.entity).despawn_recursive();");
    handle_death.line("    }");

    // Health plugin
    scope.raw("");
    scope.raw("/// Plugin that registers health systems");
    let _plugin = scope
        .new_struct("HealthPlugin")
        .vis("pub")
        .derive("Default");

    let plugin_impl = scope.new_impl("HealthPlugin");
    plugin_impl.impl_trait("Plugin");

    let build_fn = plugin_impl
        .new_fn("build")
        .arg_ref_self()
        .arg("app", "&mut App");

    build_fn.line("app");
    build_fn.line("            .add_event::<DamageEvent>()");
    build_fn.line("            .add_event::<DeathEvent>()");
    build_fn.line("            .add_systems(Update, (process_damage, handle_death).chain());");

    let code = scope.to_string();
    format_code(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_health_systems() {
        let result = generate_health_systems();
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.contains("struct Health"));
        assert!(code.contains("struct DamageEvent"));
        assert!(code.contains("struct DeathEvent"));
        assert!(code.contains("fn process_damage"));
        assert!(code.contains("fn handle_death"));
        assert!(code.contains("HealthPlugin"));
    }
}
