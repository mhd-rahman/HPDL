//! Convert AST nodes to hpdl_model types.

use crate::{ast::*, Error};
use hpdl_model::*;
use std::collections::BTreeMap;

/// Convert a FileNode to a PlanetDefinition.
pub fn file_to_planet_definition(file: FileNode) -> Result<PlanetDefinition, Error> {
    // Extract fields from planet
    let fields = fields_to_map(file.planet.fields)?;

    // Build CelestialBody
    let id = extract_string_field(&fields, "id")?;
    let body_id = BodyId::new(id).map_err(|e| Error::parse(format!("Invalid body ID: {}", e)))?;

    // Extract physical properties
    let physical = extract_physical_properties(&fields)?;

    // Extract traits
    let traits = extract_traits(&fields)?;

    // Extract modules (empty for now, will be implemented with full module support)
    let modules = extract_modules(&fields)?;

    // Extract statuses
    let statuses = extract_statuses(&fields)?;

    // Extract actions
    let actions = extract_actions(&fields)?;

    // Extract migrations
    let migrations = extract_migrations(&fields)?;

    // Create celestial body
    let body = CelestialBody::new(
        body_id, physical, traits, modules, statuses, actions, migrations,
    )
    .map_err(|e| Error::parse(format!("Invalid celestial body: {}", e)))?;

    // Create planet definition
    PlanetDefinition::new(file.language.version, body)
        .map_err(|e| Error::parse(format!("Invalid planet definition: {}", e)))
}

/// Convert a list of fields to a map for easier lookup.
fn fields_to_map(fields: Vec<FieldNode>) -> Result<BTreeMap<String, ValueNode>, Error> {
    let mut map = BTreeMap::new();

    for field in fields {
        if map.insert(field.name.clone(), field.value).is_some() {
            return Err(Error::parse(format!("Duplicate field: {}", field.name)));
        }
    }

    Ok(map)
}

/// Extract a string field.
fn extract_string_field(fields: &BTreeMap<String, ValueNode>, name: &str) -> Result<String, Error> {
    match fields.get(name) {
        Some(ValueNode::String { value, .. }) => Ok(value.clone()),
        Some(_) => Err(Error::parse(format!("Field '{}' must be a string", name))),
        None => Err(Error::parse(format!("Missing required field: {}", name))),
    }
}

/// Extract physical properties.
fn extract_physical_properties(
    fields: &BTreeMap<String, ValueNode>,
) -> Result<PhysicalProperties, Error> {
    let physical_value = fields.get("physical");

    let Some(physical_val) = physical_value else {
        // Physical properties are optional, return default
        return Ok(PhysicalProperties::default());
    };

    match physical_val {
        ValueNode::Object { fields, .. } => {
            let phys_map = fields_to_map(fields.clone())?;

            let mass_kg = extract_optional_u128(&phys_map, "mass_kg")?;
            let radius_m = extract_optional_u128(&phys_map, "radius_m")?;
            let surface_gravity_mms2 = extract_optional_u64(&phys_map, "surface_gravity_mms2")?;
            let orbital_period_s = extract_optional_u64(&phys_map, "orbital_period_s")?;
            let rotation_period_s = extract_optional_i64(&phys_map, "rotation_period_s")?;

            Ok(PhysicalProperties {
                mass_kg,
                radius_m,
                surface_gravity_mms2,
                orbital_period_s,
                rotation_period_s,
            })
        }
        _ => Err(Error::parse(
            "Field 'physical' must be an object".to_string(),
        )),
    }
}

/// Extract optional u128 field.
fn extract_optional_u128(
    fields: &BTreeMap<String, ValueNode>,
    name: &str,
) -> Result<Option<u128>, Error> {
    match fields.get(name) {
        Some(ValueNode::Integer { value, .. }) => {
            let narrowed = u128::try_from(*value).map_err(|_| {
                Error::parse(format!(
                    "Field '{}' must be a non-negative integer within u128 range",
                    name
                ))
            })?;
            Ok(Some(narrowed))
        }
        Some(_) => Err(Error::parse(format!("Field '{}' must be an integer", name))),
        None => Ok(None),
    }
}

/// Extract optional u64 field.
fn extract_optional_u64(
    fields: &BTreeMap<String, ValueNode>,
    name: &str,
) -> Result<Option<u64>, Error> {
    match fields.get(name) {
        Some(ValueNode::Integer { value, .. }) => {
            let narrowed = u64::try_from(*value).map_err(|_| {
                Error::parse(format!(
                    "Field '{}' must be a non-negative integer within u64 range",
                    name
                ))
            })?;
            Ok(Some(narrowed))
        }
        Some(_) => Err(Error::parse(format!("Field '{}' must be an integer", name))),
        None => Ok(None),
    }
}

/// Extract optional i64 field.
fn extract_optional_i64(
    fields: &BTreeMap<String, ValueNode>,
    name: &str,
) -> Result<Option<i64>, Error> {
    match fields.get(name) {
        Some(ValueNode::Integer { value, .. }) => {
            let narrowed = i64::try_from(*value).map_err(|_| {
                Error::parse(format!(
                    "Field '{}' must be an integer within i64 range",
                    name
                ))
            })?;
            Ok(Some(narrowed))
        }
        Some(_) => Err(Error::parse(format!("Field '{}' must be an integer", name))),
        None => Ok(None),
    }
}

/// Extract traits.
fn extract_traits(fields: &BTreeMap<String, ValueNode>) -> Result<TraitSet, Error> {
    let Some(traits_val) = fields.get("traits") else {
        return Ok(TraitSet::empty());
    };

    match traits_val {
        ValueNode::Object { fields, .. } => {
            let trait_map = fields_to_map(fields.clone())?;

            // Extract tags
            let tags = if let Some(ValueNode::List { elements, .. }) = trait_map.get("tags") {
                let mut tag_vec = Vec::new();
                for elem in elements {
                    if let ValueNode::String { value, .. } = elem {
                        tag_vec.push(value.clone());
                    } else {
                        return Err(Error::parse("Trait tags must be strings".to_string()));
                    }
                }
                tag_vec
            } else {
                Vec::new()
            };

            TraitSet::new(tags).map_err(|e| Error::parse(format!("Invalid trait set: {}", e)))
        }
        _ => Err(Error::parse("Field 'traits' must be an object".to_string())),
    }
}

/// Extract modules (placeholder - returns empty vec).
fn extract_modules(_fields: &BTreeMap<String, ValueNode>) -> Result<Vec<ModuleRef>, Error> {
    // TODO: Implement module extraction when needed
    Ok(Vec::new())
}

/// Extract statuses (placeholder - returns empty vec).
fn extract_statuses(
    _fields: &BTreeMap<String, ValueNode>,
) -> Result<Vec<StatusDeclaration>, Error> {
    // TODO: Implement status extraction when needed
    Ok(Vec::new())
}

/// Extract actions (placeholder - returns empty vec).
fn extract_actions(_fields: &BTreeMap<String, ValueNode>) -> Result<Vec<ActionDeclaration>, Error> {
    // TODO: Implement action extraction when needed
    Ok(Vec::new())
}

/// Extract migrations (placeholder - returns empty vec).
fn extract_migrations(
    _fields: &BTreeMap<String, ValueNode>,
) -> Result<Vec<MigrationDeclaration>, Error> {
    // TODO: Implement migration extraction when needed
    Ok(Vec::new())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_fields_to_map() {
        let fields = vec![FieldNode {
            name: "id".to_string(),
            value: ValueNode::String {
                value: "test".to_string(),
                span: crate::SourceSpan::new(0, 4),
            },
            span: crate::SourceSpan::new(0, 4),
        }];

        let map = fields_to_map(fields).unwrap();
        assert!(map.contains_key("id"));
    }

    #[test]
    fn test_extract_string_field() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "id".to_string(),
            ValueNode::String {
                value: "test".to_string(),
                span: crate::SourceSpan::new(0, 4),
            },
        );

        let result = extract_string_field(&fields, "id").unwrap();
        assert_eq!(result, "test");
    }
}
