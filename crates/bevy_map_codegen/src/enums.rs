//! Enum code generation
//!
//! Generates Rust enums from schema enum definitions with appropriate
//! derives and trait implementations.

use bevy_map_schema::Schema;
use codegen::Scope;

use crate::{format_code, to_snake_case, CodegenError};

/// Generate enum definitions for all enums in the schema
pub fn generate_enums(schema: &Schema) -> Result<String, CodegenError> {
    if schema.enums.is_empty() {
        return Ok(generate_empty_enums_module());
    }

    let mut scope = Scope::new();

    // Add imports (codegen puts these at the top automatically)
    scope.import("std::str", "FromStr");

    // Add comments (after imports in output)
    scope.raw("");
    scope.raw("// Auto-generated enum definitions from schema");
    scope.raw("// This file is regenerated when you save your map project.");
    scope.raw("// Do not edit manually - your changes will be overwritten!");
    scope.raw("");

    // Generate each enum
    let mut sorted_enums: Vec<_> = schema.enums.iter().collect();
    sorted_enums.sort_by_key(|(name, _)| *name);

    for (name, variants) in sorted_enums {
        generate_enum(&mut scope, name, variants);
    }

    let code = scope.to_string();
    format_code(&code)
}

/// Generate a single enum with FromStr implementation
fn generate_enum(scope: &mut Scope, name: &str, variants: &[String]) {
    // Create the enum
    let e = scope
        .new_enum(name)
        .derive("Debug")
        .derive("Clone")
        .derive("Copy")
        .derive("PartialEq")
        .derive("Eq")
        .derive("Hash")
        .derive("Default");

    // Add variants - first variant is default
    for (i, variant) in variants.iter().enumerate() {
        let v = e.new_variant(variant);
        if i == 0 {
            v.annotation("#[default]");
        }
    }

    // Generate FromStr implementation
    let from_str_impl = scope.new_impl(name);
    from_str_impl.impl_trait("FromStr");
    from_str_impl.associate_type("Err", "String");

    // Build the match arms
    let mut match_body = String::from("match s {\n");
    for variant in variants {
        // Match both the variant name and snake_case version
        let snake = to_snake_case(variant);
        match_body.push_str(&format!(
            "            \"{}\" | \"{}\" => Ok({}::{}),\n",
            variant, snake, name, variant
        ));
    }
    match_body.push_str(&format!(
        "            _ => Err(format!(\"Unknown {} variant: {{}}\", s)),\n",
        name
    ));
    match_body.push_str("        }");

    from_str_impl
        .new_fn("from_str")
        .arg("s", "&str")
        .ret("Result<Self, Self::Err>")
        .line(match_body);

    // Generate Display implementation
    let display_impl = scope.new_impl(name);
    display_impl.impl_trait("std::fmt::Display");

    let mut display_body = String::from("match self {\n");
    for variant in variants {
        display_body.push_str(&format!(
            "            {}::{} => write!(f, \"{}\"),\n",
            name, variant, variant
        ));
    }
    display_body.push_str("        }");

    display_impl
        .new_fn("fmt")
        .arg_ref_self()
        .arg("f", "&mut std::fmt::Formatter<'_>")
        .ret("std::fmt::Result")
        .line(display_body);

    // Generate an impl block with helper methods
    let helper_impl = scope.new_impl(name);

    // all() method returning all variants
    let mut all_body = String::from("&[\n");
    for variant in variants {
        all_body.push_str(&format!("            {}::{},\n", name, variant));
    }
    all_body.push_str("        ]");

    helper_impl
        .new_fn("all")
        .vis("pub")
        .ret(format!("&'static [{}]", name))
        .line(all_body);
}

/// Generate an empty enums module when there are no enums
fn generate_empty_enums_module() -> String {
    r#"//! Auto-generated enum definitions from schema
//!
//! No enums are defined in this schema.
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_schema() -> Schema {
        let mut schema = Schema::default();
        schema.enums.insert(
            "ItemType".to_string(),
            vec![
                "Weapon".to_string(),
                "Armor".to_string(),
                "Consumable".to_string(),
            ],
        );
        schema.enums.insert(
            "Direction".to_string(),
            vec![
                "Up".to_string(),
                "Down".to_string(),
                "Left".to_string(),
                "Right".to_string(),
            ],
        );
        schema
    }

    #[test]
    fn test_generate_enums() {
        let schema = make_test_schema();
        let result = generate_enums(&schema);
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.contains("enum ItemType"));
        assert!(code.contains("enum Direction"));
        assert!(code.contains("Weapon"));
        assert!(code.contains("Armor"));
        assert!(code.contains("impl FromStr"));
        assert!(code.contains("fn all()"));
    }

    #[test]
    fn test_generate_empty_enums() {
        let schema = Schema::default();
        let result = generate_enums(&schema);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No enums are defined"));
    }
}
