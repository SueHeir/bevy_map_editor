//! Entity instance for placed objects in the world

use crate::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// An entity placed in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInstance {
    /// Unique identifier for this instance
    pub id: Uuid,
    /// Type name (e.g., "NPC", "Enemy", "Chest")
    pub type_name: String,
    /// Position in world coordinates [x, y]
    pub position: [f32; 2],
    /// If this is an instance of a template, the template ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<Uuid>,
    /// Property overrides (for template instances) or direct properties
    #[serde(default)]
    pub properties: HashMap<String, Value>,
    /// Instance-level component overrides (override type config values)
    #[serde(default, skip_serializing_if = "ComponentOverrides::is_empty")]
    pub component_overrides: ComponentOverrides,
}

/// Instance-level overrides for component configurations
///
/// Each field is optional. When present, it overrides the corresponding
/// value from the entity type's configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentOverrides {
    /// Physics-related overrides
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physics: Option<PhysicsOverrides>,
    /// Input-related overrides
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<InputOverrides>,
    /// Sprite-related overrides
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite: Option<SpriteOverrides>,
}

impl ComponentOverrides {
    /// Check if all overrides are empty (nothing is overridden)
    pub fn is_empty(&self) -> bool {
        self.physics.as_ref().map_or(true, |p| p.is_empty())
            && self.input.as_ref().map_or(true, |i| i.is_empty())
            && self.sprite.as_ref().map_or(true, |s| s.is_empty())
    }

    /// Clear all overrides
    pub fn clear(&mut self) {
        self.physics = None;
        self.input = None;
        self.sprite = None;
    }
}

/// Physics-related instance overrides
///
/// These override values from the entity type's PhysicsConfig.
/// Note: body_type and collider shape are typically not overridable
/// per-instance as they fundamentally change the entity behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhysicsOverrides {
    /// Override gravity scale (1.0 = normal, 0.0 = no gravity)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gravity_scale: Option<f32>,
    /// Override friction coefficient
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friction: Option<f32>,
    /// Override restitution/bounciness
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restitution: Option<f32>,
    /// Override linear damping
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linear_damping: Option<f32>,
}

impl PhysicsOverrides {
    /// Check if no physics values are overridden
    pub fn is_empty(&self) -> bool {
        self.gravity_scale.is_none()
            && self.friction.is_none()
            && self.restitution.is_none()
            && self.linear_damping.is_none()
    }
}

/// Input-related instance overrides
///
/// These override values from the entity type's InputConfig.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputOverrides {
    /// Override movement speed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    /// Override jump force
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jump_force: Option<f32>,
    /// Override acceleration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceleration: Option<f32>,
    /// Override deceleration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deceleration: Option<f32>,
    /// Override max fall speed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_fall_speed: Option<f32>,
}

impl InputOverrides {
    /// Check if no input values are overridden
    pub fn is_empty(&self) -> bool {
        self.speed.is_none()
            && self.jump_force.is_none()
            && self.acceleration.is_none()
            && self.deceleration.is_none()
            && self.max_fall_speed.is_none()
    }
}

/// Sprite-related instance overrides
///
/// These override values from the entity type's SpriteConfig.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpriteOverrides {
    /// Override sprite scale
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    /// Override default animation name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_animation: Option<String>,
}

impl SpriteOverrides {
    /// Check if no sprite values are overridden
    pub fn is_empty(&self) -> bool {
        self.scale.is_none() && self.default_animation.is_none()
    }
}

impl EntityInstance {
    /// Create a new entity instance
    pub fn new(type_name: String, position: [f32; 2]) -> Self {
        Self {
            id: Uuid::new_v4(),
            type_name,
            position,
            template_id: None,
            properties: HashMap::new(),
            component_overrides: ComponentOverrides::default(),
        }
    }

    /// Create an entity instance from a template
    pub fn from_template(template_id: Uuid, type_name: String, position: [f32; 2]) -> Self {
        Self {
            id: Uuid::new_v4(),
            type_name,
            position,
            template_id: Some(template_id),
            properties: HashMap::new(),
            component_overrides: ComponentOverrides::default(),
        }
    }

    /// Get a display name for this entity
    pub fn get_display_name(&self) -> String {
        self.properties
            .get("name")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{} ({})", self.type_name, &self.id.to_string()[..8]))
    }

    /// Get a string property
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.properties.get(key).and_then(|v| v.as_string())
    }

    /// Set a string property
    pub fn set_string(&mut self, key: &str, value: String) {
        self.properties
            .insert(key.to_string(), Value::String(value));
    }

    /// Get an integer property
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.properties.get(key).and_then(|v| v.as_int())
    }

    /// Set an integer property
    pub fn set_int(&mut self, key: &str, value: i64) {
        self.properties.insert(key.to_string(), Value::Int(value));
    }

    /// Get a float property
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.properties.get(key).and_then(|v| v.as_float())
    }

    /// Set a float property
    pub fn set_float(&mut self, key: &str, value: f64) {
        self.properties.insert(key.to_string(), Value::Float(value));
    }

    /// Get a boolean property
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.properties.get(key).and_then(|v| v.as_bool())
    }

    /// Set a boolean property
    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.properties.insert(key.to_string(), Value::Bool(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_instance() {
        let mut entity = EntityInstance::new("NPC".to_string(), [100.0, 200.0]);
        entity.set_string("name", "Guard".to_string());
        entity.set_int("health", 100);

        assert_eq!(entity.get_string("name"), Some("Guard"));
        assert_eq!(entity.get_int("health"), Some(100));
        assert_eq!(entity.get_display_name(), "Guard");
    }

    #[test]
    fn test_entity_from_template() {
        let template_id = Uuid::new_v4();
        let entity = EntityInstance::from_template(template_id, "Enemy".to_string(), [50.0, 50.0]);

        assert_eq!(entity.template_id, Some(template_id));
        assert_eq!(entity.type_name, "Enemy");
    }
}
