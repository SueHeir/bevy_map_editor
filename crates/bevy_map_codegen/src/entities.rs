//! Entity struct code generation
//!
//! Generates Rust structs from schema type definitions with appropriate
//! derives and attributes for use with bevy_map_runtime.

use bevy_map_schema::{PropType, PropertyDef, Schema, TypeDef};
use codegen::Scope;

use crate::{format_code, to_snake_case, CodegenError};

/// Generate entity structs for all placeable data types in the schema
pub fn generate_entities(schema: &Schema) -> Result<String, CodegenError> {
    let mut scope = Scope::new();

    // Add imports (codegen puts these at the top automatically)
    scope.import("bevy::prelude", "*");
    scope.import("bevy_map_runtime::prelude", "*");

    // Add comment (after imports in output)
    scope.raw("");
    scope.raw("// Auto-generated entity structs from schema");
    scope.raw("// This file is regenerated when you save your map project.");
    scope.raw("// Do not edit manually - your changes will be overwritten!");
    scope.raw("");

    // Generate structs for each placeable data type
    for (name, type_def) in &schema.data_types {
        if type_def.placeable {
            generate_entity_struct(&mut scope, name, type_def, schema);
        }
    }

    let code = scope.to_string();
    format_code(&code)
}

/// Generate a single entity struct
fn generate_entity_struct(scope: &mut Scope, name: &str, type_def: &TypeDef, schema: &Schema) {
    // Generate struct manually using raw to support field-level attributes
    // The codegen crate's Struct::field() doesn't support per-field attributes

    scope.raw("#[derive(Component, Debug, Clone, Default)]");
    scope.raw(format!("#[map_entity(type_name = \"{}\")]", name));
    scope.raw(format!("pub struct {} {{", name));

    // Add fields for each property
    for prop in &type_def.properties {
        let rust_type = prop_type_to_rust(&prop.prop_type, prop, schema);
        let field_name = to_snake_case(&prop.name);

        // Add map_prop attribute
        if let Some(ref default) = prop.default {
            let default_str = format_default_value(default, &prop.prop_type);
            scope.raw(format!("    #[map_prop(default = {})]", default_str));
        } else {
            scope.raw("    #[map_prop]");
        }
        scope.raw(format!("    pub {}: {},", field_name, rust_type));
    }

    scope.raw("}");
    scope.raw("");
}

/// Convert a schema property type to a Rust type string
fn prop_type_to_rust(prop_type: &PropType, prop: &PropertyDef, schema: &Schema) -> String {
    #[allow(deprecated)]
    match prop_type {
        PropType::String | PropType::Multiline => "String".to_string(),
        PropType::Int => "i32".to_string(),
        PropType::Float => "f32".to_string(),
        PropType::Bool => "bool".to_string(),
        PropType::Enum => {
            if let Some(ref enum_type) = prop.enum_type {
                // Check if enum exists in schema
                if schema.enums.contains_key(enum_type) {
                    enum_type.clone()
                } else {
                    "String".to_string() // Fallback to string if enum not found
                }
            } else {
                "String".to_string()
            }
        }
        PropType::Ref => "Option<uuid::Uuid>".to_string(),
        PropType::Array => {
            if let Some(ref item_type) = prop.item_type {
                format!("Vec<{}>", item_type)
            } else {
                "Vec<serde_json::Value>".to_string()
            }
        }
        PropType::Embedded => {
            if let Some(ref embedded_type) = prop.embedded_type {
                embedded_type.clone()
            } else {
                "serde_json::Value".to_string()
            }
        }
        PropType::Point => "[f32; 2]".to_string(),
        PropType::Color => "[f32; 4]".to_string(),
        PropType::Sprite => "String".to_string(), // Deprecated
        PropType::Dialogue => "String".to_string(),
    }
}

/// Format a default value for use in an attribute
fn format_default_value(value: &serde_json::Value, prop_type: &PropType) -> String {
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                format!("{:.1}", f)
            } else {
                "0".to_string()
            }
        }
        serde_json::Value::String(s) => {
            // For enums, just use the variant name
            #[allow(deprecated)]
            if matches!(prop_type, PropType::Enum) {
                format!("\"{}\"", s)
            } else {
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
        }
        serde_json::Value::Array(_) => "[]".to_string(),
        serde_json::Value::Object(_) => "{}".to_string(),
    }
}

/// Generate a simple re-export module for entities
pub fn generate_entities_mod() -> String {
    r#"//! Entity component definitions
//!
//! This module re-exports all generated entity structs.

mod entities;
pub use entities::*;
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_schema() -> Schema {
        let mut schema = Schema::default();

        // Add an enum
        schema.enums.insert(
            "ItemType".to_string(),
            vec!["Weapon".to_string(), "Armor".to_string()],
        );

        // Add a placeable type
        let mut player_type = TypeDef::default();
        player_type.placeable = true;
        player_type.properties = vec![
            PropertyDef {
                name: "health".to_string(),
                prop_type: PropType::Int,
                required: true,
                default: Some(serde_json::json!(100)),
                min: None,
                max: None,
                show_if: None,
                enum_type: None,
                ref_type: None,
                item_type: None,
                embedded_type: None,
            },
            PropertyDef {
                name: "name".to_string(),
                prop_type: PropType::String,
                required: false,
                default: Some(serde_json::json!("")),
                min: None,
                max: None,
                show_if: None,
                enum_type: None,
                ref_type: None,
                item_type: None,
                embedded_type: None,
            },
        ];
        schema.data_types.insert("Player".to_string(), player_type);

        schema
    }

    #[test]
    fn test_generate_entities() {
        let schema = make_test_schema();
        let result = generate_entities(&schema);
        assert!(result.is_ok(), "generate_entities failed: {:?}", result);

        let code = result.unwrap();
        assert!(code.contains("struct Player"));
        assert!(code.contains("health"));
        assert!(code.contains("name"));
        assert!(code.contains("#[map_entity"));
    }

    #[test]
    fn test_prop_type_to_rust() {
        let schema = Schema::default();
        let prop = PropertyDef {
            name: "test".to_string(),
            prop_type: PropType::Int,
            required: false,
            default: None,
            min: None,
            max: None,
            show_if: None,
            enum_type: None,
            ref_type: None,
            item_type: None,
            embedded_type: None,
        };

        assert_eq!(prop_type_to_rust(&PropType::Int, &prop, &schema), "i32");
        assert_eq!(prop_type_to_rust(&PropType::Float, &prop, &schema), "f32");
        assert_eq!(prop_type_to_rust(&PropType::Bool, &prop, &schema), "bool");
        assert_eq!(
            prop_type_to_rust(&PropType::String, &prop, &schema),
            "String"
        );
        assert_eq!(
            prop_type_to_rust(&PropType::Point, &prop, &schema),
            "[f32; 2]"
        );
        assert_eq!(
            prop_type_to_rust(&PropType::Color, &prop, &schema),
            "[f32; 4]"
        );
    }
}
